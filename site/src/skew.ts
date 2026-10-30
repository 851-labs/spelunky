// Version skew (docs/web-deploy.md, "Version skew"): a page loads some of its code later, as it's needed (the
// wiki's search index, hover cards, popups and live clips' player), and a deploy since it loaded may have
// removed that code. build.sh carries the files of the builds before it, so this is for what's older, or a
// carry that failed: when a chunk fails to load (Vite's vite:preloadError, which every lazy import of the
// site's goes through), the page asks the live site which build it's on, and reloads, once for that build,
// if it's another: the reload gets the new one's code. The same build means the network failed, which a
// reload wouldn't mend and would lose the reader's place to: that's left to the code that asked (loadOnce
// tries again).
//
// A build is named by its entry script, /assets/index-HASH.js, which every page loads: its hash covers every
// chunk the build can load, as each chunk's name is in the chunk that imports it, up to the entry. The landing
// page asks for it (it's always there, where the page that failed may be gone from the new build), and no
// constant in the code names the build, which would rename the chunk it's in, and so the entry and every
// chunk that imports either, at every build.

// The build a reload went to, so that a page whose new build fails too doesn't reload again and again.
const RELOADED = "spelunky-reloaded-for";
// How long a check that didn't reload holds (the same build, a build it reloaded for already, or no answer), so
// that a flaky network's failures don't each ask the live site again.
const HOLD_MS = 60_000;

const entry = (doc: Document) => doc.querySelector<HTMLScriptElement>('script[type="module"][src^="/assets/"]')?.getAttribute("src") ?? null;

export function reloadOnSkew() {
  const mine = entry(document);
  // Under `vite dev` there's no built entry to compare.
  if (!mine) return () => {};
  let checking = false;
  let heldAt = -Infinity;
  const check = async () => {
    if (checking || performance.now() - heldAt < HOLD_MS) return;
    checking = true;
    try {
      const response = await fetch("/", { cache: "no-store" });
      const live = response.ok ? entry(new DOMParser().parseFromString(await response.text(), "text/html")) : null;
      if (live && live !== mine && sessionStorage.getItem(RELOADED) !== live) {
        sessionStorage.setItem(RELOADED, live);
        location.reload();
        return;
      }
    } catch {
      // Offline, or storage refused: the chunk's failure stands, as it would without this.
    } finally {
      checking = false;
    }
    heldAt = performance.now();
  };
  const listener = () => void check();
  window.addEventListener("vite:preloadError", listener);
  return () => window.removeEventListener("vite:preloadError", listener);
}
