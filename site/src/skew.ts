// Version skew (docs/web-deploy.md, "Version skew"): a page loads some of its code later, as it's needed (the
// wiki's search index, hover cards, popups and live clips' player), and a deploy since it loaded may have
// removed that code. build.sh carries the builds before it's files for a week, so this is for what's older,
// or a carry that failed: when a chunk fails to load (Vite's vite:preloadError, which every lazy import of
// the site's goes through) and the live page names another build than this one, the page reloads, once for
// that build, as the reload will get the new one's code. The same build is a network failure, which a reload
// wouldn't mend and would lose the reader's place to: it's left to the code that asked (loadOnce tries again).

declare const __SITE_BUILD__: string;

// The build a reload went to, so that a page whose new build fails too doesn't reload again and again.
const RELOADED = "spelunky-reloaded-for";

export function reloadOnSkew() {
  // Not under `vite dev`, which has no build to compare.
  if (!__SITE_BUILD__) return () => {};
  let checking = false;
  const check = async () => {
    if (checking) return;
    checking = true;
    try {
      const response = await fetch(location.href, { cache: "no-store" });
      if (!response.ok) return;
      const page = new DOMParser().parseFromString(await response.text(), "text/html");
      const live = page.querySelector('meta[name="spelunky-build"]')?.getAttribute("content");
      if (!live || live === __SITE_BUILD__ || sessionStorage.getItem(RELOADED) === live) return;
      sessionStorage.setItem(RELOADED, live);
      location.reload();
    } catch {
      // Offline, or storage refused: the chunk's failure stands, as it would without this.
    } finally {
      checking = false;
    }
  };
  const listener = () => void check();
  window.addEventListener("vite:preloadError", listener);
  return () => window.removeEventListener("vite:preloadError", listener);
}

// The page's head names its build, which the check above reads from the live page.
export const buildMeta = () => (__SITE_BUILD__ ? [{ name: "spelunky-build", content: __SITE_BUILD__ }] : []);
