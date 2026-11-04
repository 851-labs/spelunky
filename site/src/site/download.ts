// What the /download page (src/routes/download.tsx) makes of the release's downloads (src/build/release.ts): each
// manifest platform's system and processor, the systems in the page's order, and which one the reader's browser is
// on. Plain functions, so test-download.mjs runs them under Node.
import type { Release, ReleaseFile } from "../build/release.ts";

export type Os = "macos" | "windows" | "linux";
export type Arch = "x86_64" | "aarch64";

// The systems, in the page's order, with what each build needs (docs/releasing.md).
export const SYSTEMS: { os: Os; name: string; needs: string }[] = [
  { os: "macos", name: "macOS", needs: "macOS 11 or later, on Apple silicon or Intel." },
  { os: "windows", name: "Windows", needs: "Windows 10 or later." },
  {
    os: "linux",
    name: "Linux",
    needs: "glibc 2.31 or later (Ubuntu 20.04, Debian 11 and later), with ALSA and Vulkan.",
  },
];

// The manifest's platforms (docs/updates.md, "The manifest"). One it doesn't name, a platform added after this, is
// listed under "Other files", by its name.
const PLATFORMS: Record<string, { os: Os; arch: Arch | null; label: string }> = {
  macos: { os: "macos", arch: null, label: "Universal" },
  "windows-x86_64": { os: "windows", arch: "x86_64", label: "x64" },
  "windows-aarch64": { os: "windows", arch: "aarch64", label: "ARM64" },
  "linux-x86_64": { os: "linux", arch: "x86_64", label: "x64" },
  "linux-aarch64": { os: "linux", arch: "aarch64", label: "ARM64" },
};

// What a file is, by the manifest's format, as the page says it.
const FORMATS: Record<string, string> = { "app-zip": "zip", "dir-zip": "zip", appimage: "AppImage" };

// The release's other files (SHA256SUMS's, src/build/release.ts) by their names, spelunky-VERSION-TARGET.EXT, as
// build-native.yml names them (docs/releasing.md): TARGET's platform, as the manifest would name it.
const TARGETS: Record<string, string> = {
  macos: "macos",
  "windows-x64": "windows-x86_64",
  "windows-arm64": "windows-aarch64",
  "linux-x64": "linux-x86_64",
  "linux-arm64": "linux-aarch64",
};

export interface Download {
  // The manifest's platform; for another file, its target's (an unknown target's own name).
  platform: string;
  url: string;
  file: string;
  size: number;
  // Null for a platform the page doesn't know.
  os: Os | null;
  arch: Arch | null;
  // "x64", "Universal", or an unknown platform's own name.
  label: string;
  kind: string;
}

export interface Downloads {
  // `more`: the system's files in other formats than the apps update from, the .dmg and the tarballs.
  systems: { os: Os; name: string; needs: string; downloads: Download[]; more: Download[] }[];
  other: Download[];
}

// x64 before ARM64, as the manifest's keys sort the other way.
const ARCH_ORDER: (Arch | null)[] = [null, "x86_64", "aarch64"];

export function downloads(release: Release): Downloads {
  const entry = (platform: string, url: string, file: string, size: number, kind: string): Download => {
    const known = PLATFORMS[platform];
    return { platform, url, file, size, os: known?.os ?? null, arch: known?.arch ?? null, label: known?.label ?? platform, kind };
  };
  const all = release.assets.map((asset) =>
    entry(asset.platform, asset.url, asset.file, asset.size, FORMATS[asset.format] ?? extension(asset.file)),
  );
  const byArch = (a: Download, b: Download) => ARCH_ORDER.indexOf(a.arch) - ARCH_ORDER.indexOf(b.arch);
  const prefix = `spelunky-${release.version}-`;
  const more = release.files
    .map((file) => {
      const name = file.file;
      const target = name.startsWith(prefix) ? name.slice(prefix.length).replace(/\.(tar\.gz|[^.]+)$/, "") : name;
      return entry(TARGETS[target] ?? target, file.url, name, file.size, extension(name));
    })
    .sort(byArch);
  const systems = SYSTEMS.map((system) => ({
    ...system,
    downloads: all.filter((download) => download.os === system.os).sort(byArch),
    more: more.filter((download) => download.os === system.os),
  })).filter((system) => system.downloads.length > 0);
  // A file of a system without the apps' own (none is), or a platform the page doesn't know: by its name.
  const listed = new Set(systems.map((system) => system.os));
  return { systems, other: [...all, ...more].filter((download) => download.os === null || !listed.has(download.os)) };
}

// "tar.gz", "dmg": a file's extension, both of a tarball's.
function extension(file: string): string {
  return /\.(tar\.gz|[^.]+)$/.exec(file)?.[1] ?? file;
}

// The download a system's reader gets first: the one for their processor, or (unknown) the first, x64, which
// Windows on ARM runs too.
export function pick(list: Download[], arch: Arch | null): Download | undefined {
  return list.find((download) => download.arch === null || download.arch === arch) ?? list[0];
}

// Decimal megabytes, as the systems' own file browsers show them: 26388724 is "26.4 MB".
export function size(bytes: number): string {
  // (Rounded first: 999,700 bytes is "1.0 MB", not "1000 KB".)
  if (Math.round(bytes / 1e3) < 1000) return `${Math.max(1, Math.round(bytes / 1e3))} KB`;
  return `${(bytes / 1e6).toFixed(1)} MB`;
}

export interface System {
  os: Os;
  arch: Arch | null;
}

// What the browser says of its system: navigator.userAgentData where there is one (Chromium's), and the user agent
// and navigator.platform otherwise. `architecture` is userAgentData's high-entropy value ("x86", "arm"), which
// only Chromium gives, and asynchronously.
export interface BrowserSystem {
  userAgent: string;
  platform: string;
  uaPlatform?: string;
  architecture?: string;
  // userAgentData's "32" or "64", with `architecture`.
  bitness?: string;
  // navigator.maxTouchPoints: an iPad's Safari says it's a Mac.
  touchPoints: number;
}

// Null for a phone, a tablet or a Chromebook, which none of the downloads are for, and a system it can't tell.
export function detect(browser: BrowserSystem): System | null {
  const { userAgent: ua, touchPoints } = browser;
  const platform = browser.uaPlatform || browser.platform;
  if (/Android|iPhone|iPad|iPod|CrOS/i.test(ua) || /^(Android|iOS|Chrome OS)$/i.test(browser.uaPlatform ?? "")) {
    return null;
  }
  let os: Os;
  if (/^Win|Windows/i.test(platform) || /Windows/.test(ua)) os = "windows";
  else if (/^Mac|macOS/i.test(platform) || /Macintosh/.test(ua)) {
    // iPadOS's Safari asks for the desktop site as a Mac; only an iPad of the two has a touch screen.
    if (touchPoints > 1) return null;
    os = "macos";
  } else if (/Linux|X11/i.test(platform) || /Linux|X11/.test(ua)) os = "linux";
  else return null;
  return { os, arch: arch(browser) };
}

function arch({ architecture, bitness, userAgent: ua, platform }: BrowserSystem): Arch | null {
  // A 32-bit system, which no build is for.
  if (bitness === "32") return null;
  if (architecture === "arm") return "aarch64";
  if (architecture === "x86") return "x86_64";
  const text = `${ua} ${platform}`;
  if (/aarch64|arm64|armv8/i.test(text)) return "aarch64";
  if (/x86_64|x86-64|amd64|Win64|WOW64|x64/i.test(text)) return "x86_64";
  return null;
}

// The browser's own, in the page: userAgentData's architecture, where Chromium gives it.
export async function browserSystem(nav: Navigator): Promise<BrowserSystem> {
  const data = (
    nav as Navigator & {
      userAgentData?: { platform?: string; getHighEntropyValues?(hints: string[]): Promise<{ architecture?: string; bitness?: string }> };
    }
  ).userAgentData;
  let values: { architecture?: string; bitness?: string } | undefined;
  try {
    values = await data?.getHighEntropyValues?.(["architecture", "bitness"]);
  } catch {
    // Refused (a privacy setting): the user agent's, then.
  }
  return {
    userAgent: nav.userAgent,
    platform: nav.platform ?? "",
    uaPlatform: data?.platform,
    architecture: values?.architecture,
    bitness: values?.bitness,
    touchPoints: nav.maxTouchPoints ?? 0,
  };
}
