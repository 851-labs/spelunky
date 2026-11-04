// The /download page's logic (src/build/release.ts, src/site/download.ts), which site/test.sh runs, as does a
// change to them locally: the update manifest it lists the files from, read or refused, and the release's
// SHA256SUMS and pinned sizes it lists the other formats from; the systems they're grouped into, in order; and
// which system a browser's on, from what real browsers say.
//
//   node test-download.mjs
import assert from "node:assert/strict";
import { parseFiles, parseRelease } from "./src/build/release.ts";
import { detect, downloads, pick, size } from "./src/site/download.ts";

let failures = 0;
function test(name, run) {
  try {
    run();
    console.log(`ok   ${name}`);
  } catch (error) {
    failures++;
    console.error(`FAIL ${name}\n${error.stack ?? error}`);
  }
}

const SHA = (c) => c.repeat(64);
const asset = (file, format, sha = "a") => ({
  url: `https://github.com/851-labs/spelunky/releases/download/v0.1.19/${file}`,
  sha256: SHA(sha),
  size: 17875061,
  format,
});
// As release-manifest signs it (docs/updates.md): the manifest a string in the file, its keys sorted.
const MANIFEST = {
  version: "0.1.19",
  notes: "https://github.com/851-labs/spelunky/releases/tag/v0.1.19",
  assets: {
    "linux-aarch64": asset("spelunky-0.1.19-linux-arm64.AppImage", "appimage", "b"),
    "linux-x86_64": asset("spelunky-0.1.19-linux-x64.AppImage", "appimage", "c"),
    macos: asset("spelunky-0.1.19-macos.zip", "app-zip", "d"),
    "windows-aarch64": asset("spelunky-0.1.19-windows-arm64.zip", "dir-zip", "e"),
    "windows-x86_64": asset("spelunky-0.1.19-windows-x64.zip", "dir-zip", "f"),
  },
};
const file = (manifest) => JSON.stringify({ manifest: JSON.stringify(manifest), signature: "00" });
const edited = (change) => {
  const manifest = structuredClone(MANIFEST);
  change(manifest);
  return file(manifest);
};

test("a release's manifest gives each platform's file, size and SHA-256", () => {
  const release = parseRelease(file(MANIFEST));
  assert.equal(release.version, "0.1.19");
  assert.equal(release.notes, MANIFEST.notes);
  const mac = release.assets.find((a) => a.platform === "macos");
  assert.deepEqual(mac, { platform: "macos", ...MANIFEST.assets.macos, file: "spelunky-0.1.19-macos.zip" });
  // A prerelease's version, as the apps take it.
  assert.equal(parseRelease(edited((m) => (m.version = "0.2.0-rc.1"))).version, "0.2.0-rc.1");
});

test("a manifest the page can't list fails the build, naming what's wrong", () => {
  const refused = [
    ["{}", /its manifest isn't a string/],
    [JSON.stringify({ manifest: "{" }), /its manifest isn't JSON/],
    ["{", /the file isn't JSON/],
    [JSON.stringify({ manifest: "[]" }), /its manifest isn't an object/],
    [edited((m) => (m.version = "v0.1.19")), /isn't x\.y\.z/],
    [edited((m) => (m.assets.macos.url = "http://example.com/a.zip")), /isn't an HTTPS URL/],
    [edited((m) => (m.assets.macos.url = "https://")), /isn't an HTTPS URL/],
    [edited((m) => (m.notes = "releases/tag/v0.1.19")), /the notes .* isn't an HTTPS URL/],
    [edited((m) => (m.assets.macos.url = "https://example.com/")), /names no file/],
    [edited((m) => (m.assets.macos.sha256 = "ABC")), /64 hex digits/],
    [edited((m) => (m.assets.macos.size = "26 MB")), /size isn't a size/],
    [edited((m) => (m.assets.macos.size = 0)), /size isn't a size/],
    [edited((m) => delete m.assets.macos.format), /format isn't a string/],
    [edited((m) => (m.assets = {})), /no assets/],
  ];
  for (const [text, want] of refused) {
    assert.throws(() => parseRelease(text, "latest.json"), (error) => want.test(error.message) && error.message.startsWith("latest.json: "));
  }
});

test("the files are grouped by system, macOS, Windows, Linux, x64 before ARM64", () => {
  const list = downloads(parseRelease(file(MANIFEST)));
  assert.deepEqual(
    list.systems.map((s) => [s.os, s.downloads.map((d) => `${d.label} ${d.kind}`)]),
    [
      ["macos", ["Universal zip"]],
      ["windows", ["x64 zip", "ARM64 zip"]],
      ["linux", ["x64 AppImage", "ARM64 AppImage"]],
    ],
  );
  assert.deepEqual(list.other, []);
});

test("a platform added after the page is listed under its own name, and a system without files isn't", () => {
  const list = downloads(
    parseRelease(
      edited((m) => {
        delete m.assets["linux-aarch64"];
        delete m.assets["linux-x86_64"];
        m.assets["freebsd-x86_64"] = asset("spelunky-0.1.19-freebsd-x64.tar.gz", "tarball");
      }),
    ),
  );
  assert.deepEqual(list.systems.map((s) => s.os), ["macos", "windows"]);
  assert.deepEqual(list.other.map((d) => [d.label, d.kind]), [["freebsd-x86_64", "tar.gz"]]);
});

test("a reader gets their processor's file first, or x64's", () => {
  const windows = downloads(parseRelease(file(MANIFEST))).systems[1].downloads;
  assert.equal(pick(windows, "aarch64").label, "ARM64");
  assert.equal(pick(windows, "x86_64").label, "x64");
  assert.equal(pick(windows, null).label, "x64");
  const mac = downloads(parseRelease(file(MANIFEST))).systems[0].downloads;
  assert.equal(pick(mac, "aarch64").label, "Universal");
});

// The release's SHA256SUMS, as release.yml writes it (sha256sum, by name), and web-release.txt's lines.
const SUMS = [
  `${SHA("1")}  latest.json`,
  `${SHA("b")}  spelunky-0.1.19-linux-arm64.AppImage`,
  `${SHA("2")}  spelunky-0.1.19-linux-arm64.tar.gz`,
  `${SHA("c")}  spelunky-0.1.19-linux-x64.AppImage`,
  `${SHA("3")}  spelunky-0.1.19-linux-x64.tar.gz`,
  `${SHA("4")}  spelunky-0.1.19-macos.dmg`,
  `${SHA("d")}  spelunky-0.1.19-macos.zip`,
  `${SHA("e")}  spelunky-0.1.19-windows-arm64.zip`,
  `${SHA("f")}  spelunky-0.1.19-windows-x64.zip`,
  `${SHA("5")}  spelunky-web-v0.1.19.tar.gz`,
  "",
].join("\n");
const PIN = `# The comment.
tag v0.1.19
sha256 ${SHA("5")}
manifest-sha256 ${SHA("1")}
sums-sha256 ${SHA("6")}
size spelunky-0.1.19-linux-arm64.tar.gz 16705489
size spelunky-0.1.19-linux-x64.tar.gz 17551610
size spelunky-0.1.19-macos.dmg 30151290
`;
const withFiles = (sums = SUMS, pin = PIN, text = file(MANIFEST)) => {
  const release = parseRelease(text);
  release.files = parseFiles(release, sums, pin);
  return release;
};

test("the release's other files are SHA256SUMS's that the bump sized, not the manifest's, in its order", () => {
  assert.deepEqual(withFiles().files, [
    {
      url: "https://github.com/851-labs/spelunky/releases/download/v0.1.19/spelunky-0.1.19-linux-arm64.tar.gz",
      file: "spelunky-0.1.19-linux-arm64.tar.gz",
      sha256: SHA("2"),
      size: 16705489,
    },
    {
      url: "https://github.com/851-labs/spelunky/releases/download/v0.1.19/spelunky-0.1.19-linux-x64.tar.gz",
      file: "spelunky-0.1.19-linux-x64.tar.gz",
      sha256: SHA("3"),
      size: 17551610,
    },
    {
      url: "https://github.com/851-labs/spelunky/releases/download/v0.1.19/spelunky-0.1.19-macos.dmg",
      file: "spelunky-0.1.19-macos.dmg",
      sha256: SHA("4"),
      size: 30151290,
    },
  ]);
  // sha256sum's binary mode marks a name with a *.
  assert.equal(withFiles(SUMS.replace("  spelunky-0.1.19-macos.dmg", " *spelunky-0.1.19-macos.dmg")).files.length, 3);
  // A manifest's file with a size is still the manifest's, listed once.
  assert.equal(withFiles(SUMS, `${PIN}size spelunky-0.1.19-macos.zip 26388724\n`).files.length, 3);
  // And none without sizes.
  assert.deepEqual(withFiles(SUMS, "tag v0.1.19\n").files, []);
});

test("a SHA256SUMS or sizes the page can't list fails the build, naming what's wrong", () => {
  const refused = [
    [SUMS.replace("  latest.json", " latest.json"), PIN, /"[0-9a-f]+ latest.json" isn't a SHA-256/],
    [SUMS.replace(`${SHA("4")}  spelunky-0.1.19-macos.dmg`, "4  spelunky-0.1.19-macos.dmg"), PIN, /isn't a SHA-256/],
    [`${SUMS}${SHA("4")}  spelunky-0.1.19-macos.dmg\n`, PIN, /spelunky-0.1.19-macos.dmg is there twice/],
    // Another release's: the manifest's files aren't there, or with other SHA-256s.
    [SUMS.replaceAll("0.1.19", "0.1.20"), PIN, /the manifest's spelunky-0.1.19-.* isn't there/],
    [SUMS.replace(`${SHA("d")}  spelunky-0.1.19-macos.zip`, `${SHA("9")}  spelunky-0.1.19-macos.zip`), PIN, /macos.zip isn't there with the manifest's SHA-256/],
    [SUMS, `${PIN}size spelunky-0.1.19-freebsd-x64.tar.gz 1\n`, /freebsd-x64.tar.gz has a size, but SHA256SUMS hasn't it/],
    [SUMS, `${PIN}size spelunky-0.1.19-macos.dmg 30151290\n`, /macos.dmg has two sizes/],
    [SUMS, PIN.replace("30151290", "30 MB"), /isn't "size NAME BYTES"/],
    [SUMS, PIN.replace("30151290", "0"), /isn't "size NAME BYTES"/],
    [SUMS, PIN.replace(" 30151290", ""), /isn't "size NAME BYTES"/],
  ];
  for (const [sums, pin, want] of refused) {
    assert.throws(() => withFiles(sums, pin), (error) => want.test(error.message));
  }
});

test("the other formats are under their systems, x64 before ARM64; a target the page doesn't know is by its name", () => {
  const list = downloads(withFiles());
  assert.deepEqual(
    list.systems.map((s) => [s.os, s.more.map((d) => `${d.label} ${d.kind} ${d.file}`)]),
    [
      ["macos", ["Universal dmg spelunky-0.1.19-macos.dmg"]],
      ["windows", []],
      [
        "linux",
        ["x64 tar.gz spelunky-0.1.19-linux-x64.tar.gz", "ARM64 tar.gz spelunky-0.1.19-linux-arm64.tar.gz"],
      ],
    ],
  );
  assert.deepEqual(list.other, []);
  const freebsd = `${SHA("7")}  spelunky-0.1.19-freebsd-x64.tar.gz\n`;
  const other = downloads(withFiles(SUMS + freebsd, `${PIN}size spelunky-0.1.19-freebsd-x64.tar.gz 1000\n`)).other;
  assert.deepEqual(other.map((d) => [d.label, d.kind]), [["freebsd-x64", "tar.gz"]]);
  // And a system's other format, where the manifest has none of that system's files: by its name too, not a card
  // without the apps' own.
  const noLinux = edited((m) => {
    delete m.assets["linux-aarch64"];
    delete m.assets["linux-x86_64"];
  });
  const list2 = downloads(withFiles(SUMS, PIN, noLinux));
  assert.deepEqual(list2.systems.map((s) => s.os), ["macos", "windows"]);
  assert.deepEqual(list2.other.map((d) => [d.label, d.kind]), [["x64", "tar.gz"], ["ARM64", "tar.gz"]]);
});

test("sizes are decimal megabytes, as file browsers show them", () => {
  assert.equal(size(26388724), "26.4 MB");
  assert.equal(size(17000000), "17.0 MB");
  assert.equal(size(1517), "2 KB");
  assert.equal(size(999700), "1.0 MB");
});

// What real browsers say: their user agent, navigator.platform, and Chromium's userAgentData.
const browser = (userAgent, platform, extra = {}) => ({ userAgent, platform, touchPoints: 0, ...extra });
const CHROME_MAC = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/141.0.0.0 Safari/537.36";
const SAFARI_MAC = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/18.0 Safari/605.1.15";
const EDGE_WIN = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/141.0.0.0 Safari/537.36 Edg/141.0.0.0";
const FIREFOX_WIN = "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:144.0) Gecko/20100101 Firefox/144.0";
const FIREFOX_LINUX = "Mozilla/5.0 (X11; Linux x86_64; rv:144.0) Gecko/20100101 Firefox/144.0";
const CHROME_LINUX_ARM = "Mozilla/5.0 (X11; Linux aarch64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/141.0.0.0 Safari/537.36";
const IPHONE = "Mozilla/5.0 (iPhone; CPU iPhone OS 18_0 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/18.0 Mobile/15E148 Safari/604.1";
const ANDROID = "Mozilla/5.0 (Linux; Android 10; K) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/141.0.0.0 Mobile Safari/537.36";
const CHROMEBOOK = "Mozilla/5.0 (X11; CrOS x86_64 14541.0.0) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/141.0.0.0 Safari/537.36";

test("the reader's system, from what their browser says", () => {
  const cases = [
    [browser(CHROME_MAC, "MacIntel", { uaPlatform: "macOS", architecture: "arm" }), { os: "macos", arch: "aarch64" }],
    [browser(SAFARI_MAC, "MacIntel"), { os: "macos", arch: null }],
    [browser(EDGE_WIN, "Win32", { uaPlatform: "Windows", architecture: "x86" }), { os: "windows", arch: "x86_64" }],
    // Windows on ARM: Chromium's userAgentData says so, where its user agent says x64.
    [browser(EDGE_WIN, "Win32", { uaPlatform: "Windows", architecture: "arm" }), { os: "windows", arch: "aarch64" }],
    [browser(FIREFOX_WIN, "Win32"), { os: "windows", arch: "x86_64" }],
    [browser(FIREFOX_LINUX, "Linux x86_64"), { os: "linux", arch: "x86_64" }],
    [browser(CHROME_LINUX_ARM, "Linux aarch64", { uaPlatform: "Linux" }), { os: "linux", arch: "aarch64" }],
    // A 32-bit Raspberry Pi, which no build is for: no processor of its own, so it's offered x64's first.
    [browser(CHROME_LINUX_ARM, "Linux armv7l", { uaPlatform: "Linux", architecture: "arm", bitness: "32" }), { os: "linux", arch: null }],
    // Phones, tablets and Chromebooks, which no download is for.
    [browser(IPHONE, "iPhone", { touchPoints: 5 }), null],
    // iPadOS's Safari, which asks for the desktop site as a Mac.
    [browser(SAFARI_MAC, "MacIntel", { touchPoints: 5 }), null],
    [browser(ANDROID, "Linux armv81", { uaPlatform: "Android", touchPoints: 5 }), null],
    [browser(CHROMEBOOK, "Linux x86_64", { uaPlatform: "Chrome OS" }), null],
    [browser("Mozilla/5.0 (PlayStation 5)", ""), null],
  ];
  for (const [given, want] of cases) assert.deepEqual(detect(given), want, given.userAgent);
});

if (failures > 0) {
  console.error(`site/test-download.mjs: ${failures} failed`);
  process.exit(1);
}
console.log("site/test-download.mjs: all passed");
