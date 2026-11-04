// What the /download page (src/routes/download.tsx) makes of the release's downloads (src/build/release.ts): each
// manifest platform's system and processor, the systems in the page's order, and which one the reader's browser is
// on. Plain functions, so test-download.mjs runs them under Node.
import type { Release, ReleaseAsset } from "../build/release.ts";

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

export interface Download extends ReleaseAsset {
  arch: Arch | null;
  // "x64", "Universal", or an unknown platform's own name.
  label: string;
  kind: string;
}

export interface Downloads {
  systems: { os: Os; name: string; needs: string; downloads: Download[] }[];
  other: Download[];
}

// x64 before ARM64, as the manifest's keys sort the other way.
const ARCH_ORDER: (Arch | null)[] = [null, "x86_64", "aarch64"];

export function downloads(release: Release): Downloads {
  const all = release.assets.map((asset) => {
    const known = PLATFORMS[asset.platform];
    return {
      ...asset,
      os: known?.os ?? null,
      arch: known?.arch ?? null,
      label: known?.label ?? asset.platform,
      kind: FORMATS[asset.format] ?? asset.file.replace(/^.*?\.(?=[^.]+$)/, ""),
    };
  });
  const strip = ({ os: _, ...download }: Download & { os: Os | null }): Download => download;
  return {
    systems: SYSTEMS.map((system) => ({
      ...system,
      downloads: all
        .filter((download) => download.os === system.os)
        .sort((a, b) => ARCH_ORDER.indexOf(a.arch) - ARCH_ORDER.indexOf(b.arch))
        .map(strip),
    })).filter((system) => system.downloads.length > 0),
    other: all.filter((download) => download.os === null).map(strip),
  };
}

// The download a system's reader gets first: the one for their processor, or (unknown) the first, x64, which
// Windows on ARM runs too.
export function pick(list: Download[], arch: Arch | null): Download | undefined {
  return list.find((download) => download.arch === null || download.arch === arch) ?? list[0];
}

// Decimal megabytes, as the systems' own file browsers show them: 26388724 is "26.4 MB".
export function size(bytes: number): string {
  if (bytes < 1e6) return `${Math.max(1, Math.round(bytes / 1e3))} KB`;
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

function arch({ architecture, userAgent: ua, platform }: BrowserSystem): Arch | null {
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
      userAgentData?: { platform?: string; getHighEntropyValues?(hints: string[]): Promise<{ architecture?: string }> };
    }
  ).userAgentData;
  let architecture: string | undefined;
  try {
    architecture = (await data?.getHighEntropyValues?.(["architecture"]))?.architecture;
  } catch {
    // Refused (a privacy setting): the user agent's, then.
  }
  return {
    userAgent: nav.userAgent,
    platform: nav.platform ?? "",
    uaPlatform: data?.platform,
    architecture,
    touchPoints: nav.maxTouchPoints ?? 0,
  };
}
