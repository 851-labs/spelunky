// Hover cards (docs/wiki-style.md, "Hover cards"): hovering a link to a written page, focusing it from the
// keyboard, or long-pressing it on a touch screen shows a card of the page's sprite, title, lede and first
// numbers, as Wikipedia's page previews do. A tap still follows the link.
//
// One set of listeners on the document serves every link, whatever renders it (a page's body, its
// relations, the navboxes): an `<a data-slug>`. A red link is a <span>, so it has none. The cards' data
// (build/cards.ts) is its own chunk, loaded the first time a link is hovered or focused, so a page that's
// only read never loads it.
//
// The card is a tooltip: role="tooltip", and the link's aria-describedby while it shows. It stays while the
// pointer is on it, Escape closes it, and it's out of the page's flow, placed by the link inside the
// viewport, so nothing moves when it shows.
import { useEffect, useId, useLayoutEffect, useRef, useState } from "react";
import type { PointerEvent as ReactPointerEvent } from "react";
import { createPortal } from "react-dom";
import type { HoverCard } from "./build/cards";
import { Sprite } from "./components";

// How long the pointer rests on a link before its card shows, how long the card stays once the pointer
// leaves (time to move onto it), and how long a touch is held to be a long press.
const SHOW_DELAY = 350;
const HIDE_DELAY = 250;
const LONG_PRESS = 500;
// How far a held touch may move and still be a press rather than a scroll.
const PRESS_SLOP = 10;
// The card's distance from the link, and its least from the viewport's edges: past its stone frame's two
// rings and shadow (wiki.css's --stone), 8 and 16 pixels.
const GAP = 12;
const MARGIN = 20;

type Cards = Record<string, HoverCard>;

let loading: Promise<Cards> | null = null;

// The cards, from their own chunk: loaded once, and tried again after a failure (the network, say).
function loadCards(): Promise<Cards> {
  // The Worker never shows one, so its bundle needn't carry them.
  if (import.meta.env.SSR) return new Promise(() => {});
  return (loading ??= import("virtual:wiki/hover-cards").then(
    (module) => module.cards,
    (error) => {
      loading = null;
      throw error;
    },
  ));
}

interface Shown {
  link: HTMLAnchorElement;
  card: HoverCard;
  // Where the pointer was, to put the card by the line it's on when the link wraps.
  point: { x: number; y: number } | null;
}

const linkOf = (target: EventTarget | null) => (target instanceof Element ? target.closest<HTMLAnchorElement>("a[data-slug]") : null);

export function HoverCards() {
  const id = `${useId()}-hover-card`;
  const [shown, setShown] = useState<Shown | null>(null);
  const ref = useRef<HTMLDivElement>(null);
  // The card's own pointer handlers, which share the listeners' timers.
  const handlers = useRef({ enter: () => {}, leave: (_e: ReactPointerEvent) => {} });

  useEffect(() => {
    // The link whose card shows, or is about to, and the link focused from the keyboard, whose card the
    // pointer passing over it doesn't close.
    let current: HTMLAnchorElement | null = null;
    let focused: HTMLAnchorElement | null = null;
    let showTimer: ReturnType<typeof setTimeout> | undefined;
    let hideTimer: ReturnType<typeof setTimeout> | undefined;
    // A touch being held on a link, and a link that's been long-pressed, whose click (if the browser sends
    // one when the finger lifts) mustn't follow it.
    let press: { link: HTMLAnchorElement; x: number; y: number; timer: ReturnType<typeof setTimeout> } | null = null;
    let pressed: HTMLAnchorElement | null = null;

    function show(link: HTMLAnchorElement, delay: number, point: Shown["point"]) {
      clearTimeout(hideTimer);
      if (link === current) return;
      clearTimeout(showTimer);
      // Another link's card goes at once, rather than stay over the link the pointer's moved to.
      if (current) setShown(null);
      current = link;
      const cards = loadCards();
      showTimer = setTimeout(() => {
        cards.then(
          (all) => {
            const slug = link.dataset.slug!;
            if (current === link && Object.hasOwn(all, slug)) setShown({ link, card: all[slug], point });
          },
          // No card, then: the link still works, and the next hover tries again.
          () => {},
        );
      }, delay);
    }
    function close() {
      clearTimeout(showTimer);
      clearTimeout(hideTimer);
      current = null;
      setShown(null);
    }
    function hide(delay: number) {
      clearTimeout(hideTimer);
      hideTimer = setTimeout(close, delay);
    }
    handlers.current = {
      enter: () => clearTimeout(hideTimer),
      leave: (e) => void (e.pointerType !== "touch" && current !== focused && hide(HIDE_DELAY)),
    };
    const endPress = () => {
      if (press) clearTimeout(press.timer);
      press = null;
    };

    const listeners: { [K in keyof DocumentEventMap]?: (e: DocumentEventMap[K]) => void } = {
      pointerover(e) {
        const link = linkOf(e.target);
        if (link && e.pointerType !== "touch") show(link, SHOW_DELAY, { x: e.clientX, y: e.clientY });
      },
      pointerout(e) {
        const link = linkOf(e.target);
        // Leaving the link, not moving between its parts; onto the card, the card's `enter` keeps it.
        if (link && link === current && link !== focused && e.pointerType !== "touch" && !link.contains(e.relatedTarget as Node | null)) {
          hide(HIDE_DELAY);
        }
      },
      focusin(e) {
        const link = linkOf(e.target);
        // From the keyboard: a click focuses the link too, but the pointer's on it already.
        if (link && link.matches(":focus-visible")) {
          focused = link;
          show(link, 0, null);
        }
      },
      focusout(e) {
        const link = linkOf(e.target);
        if (link === focused) focused = null;
        if (link === current) hide(0);
      },
      keydown(e) {
        // Not an Escape another popup has taken (the search box's list).
        if (e.key === "Escape" && current && !e.defaultPrevented) close();
      },
      pointerdown(e) {
        pressed = null;
        endPress();
        if (ref.current?.contains(e.target as Node)) return;
        const link = linkOf(e.target);
        if (link !== current) hide(0);
        if (link && e.pointerType === "touch") {
          const timer = setTimeout(() => {
            pressed = link;
            show(link, 0, { x: e.clientX, y: e.clientY });
          }, LONG_PRESS);
          press = { link, x: e.clientX, y: e.clientY, timer };
        }
      },
      pointermove(e) {
        if (press && Math.hypot(e.clientX - press.x, e.clientY - press.y) > PRESS_SLOP) endPress();
      },
      pointerup: endPress,
      pointercancel: endPress,
      // A long press's own menu (Android's, the desktop's under touch emulation): the card instead.
      contextmenu(e) {
        const link = linkOf(e.target);
        if (!link || (link !== press?.link && link !== pressed)) return;
        e.preventDefault();
        if (link !== pressed) {
          const { x, y } = press!;
          endPress();
          pressed = link;
          show(link, 0, { x, y });
        }
      },
      click(e) {
        if (pressed && linkOf(e.target) === pressed) e.preventDefault();
        pressed = null;
      },
    };
    const all = Object.entries(listeners) as [string, EventListener][];
    for (const [type, listener] of all) document.addEventListener(type, listener);
    // Placed in the page's coordinates, so it goes when the link moves without the page: a box scrolling
    // (a wide table's), or the page reflowing to a new width. Not for a new height alone, which a phone's
    // address bar makes as the page scrolls.
    const scrolled = (e: Event) => void (current && e.target !== document && !ref.current?.contains(e.target as Node) && close());
    let width = document.documentElement.clientWidth;
    const resized = () => {
      if (document.documentElement.clientWidth === width) return;
      width = document.documentElement.clientWidth;
      if (current) close();
    };
    // Nor is it left for the page's return from the back/forward cache, after a tap on its link.
    const left = () => current && close();
    document.addEventListener("scroll", scrolled, true);
    window.addEventListener("resize", resized);
    window.addEventListener("pagehide", left);
    return () => {
      for (const [type, listener] of all) document.removeEventListener(type, listener);
      document.removeEventListener("scroll", scrolled, true);
      window.removeEventListener("resize", resized);
      window.removeEventListener("pagehide", left);
      clearTimeout(showTimer);
      clearTimeout(hideTimer);
      endPress();
    };
  }, []);

  // Described by the card while it shows.
  useEffect(() => {
    if (!shown) return;
    const { link } = shown;
    const before = link.getAttribute("aria-describedby");
    link.setAttribute("aria-describedby", before ? `${before} ${id}` : id);
    return () => {
      if (before === null) link.removeAttribute("aria-describedby");
      else link.setAttribute("aria-describedby", before);
    };
  }, [shown, id]);

  // Placed before it's painted: under the link, or over it when only that fits, its left edge at the link's;
  // where neither fits, as low as the viewport lets it, over the link. Inside the viewport (its CSS keeps it
  // shorter), so it never lengthens the page; in the page's coordinates, so it scrolls with the link.
  useLayoutEffect(() => {
    const card = ref.current;
    if (!shown || !card) return;
    const rects = [...shown.link.getClientRects()];
    const { point } = shown;
    const rect =
      (point && rects.find((r) => point.x >= r.left && point.x <= r.right && point.y >= r.top && point.y <= r.bottom)) ??
      rects[0] ??
      shown.link.getBoundingClientRect();
    const { clientWidth: width, clientHeight: height } = document.documentElement;
    const { offsetWidth: w, offsetHeight: h } = card;
    const below = rect.bottom + GAP;
    const above = rect.top - GAP - h;
    const top = below + h <= height - MARGIN ? below : above >= MARGIN ? above : Math.max(MARGIN, height - MARGIN - h);
    const left = Math.max(MARGIN, Math.min(rect.left, width - MARGIN - w));
    card.style.top = `${top + window.scrollY}px`;
    card.style.left = `${left + window.scrollX}px`;
    card.style.visibility = "visible";
  }, [shown]);

  if (!shown) return null;
  const { card } = shown;
  return createPortal(
    // Hidden until it's placed.
    <div
      ref={ref}
      id={id}
      role="tooltip"
      className="hover-card"
      style={{ visibility: "hidden" }}
      onPointerEnter={() => handlers.current.enter()}
      onPointerLeave={(e) => handlers.current.leave(e)}
    >
      {card.image && (
        <span className="hover-card-image">
          <Sprite name="" alt="" width={card.image.width} height={card.image.height} src={card.image.src} still={card.image.still ?? undefined} loading="eager" />
        </span>
      )}
      <div className="hover-card-text">
        <p className="hover-card-title">
          <strong>{card.title}</strong> <span className="hover-card-kind">{card.kind}</span>
        </p>
        <p className="hover-card-lede">{card.lede}</p>
        {card.stats.length > 0 && (
          <dl className="hover-card-stats">
            {card.stats.map(([label, text]) => (
              <div key={label}>
                <dt>{label}</dt>
                <dd>{text}</dd>
              </div>
            ))}
          </dl>
        )}
      </div>
    </div>,
    document.body,
  );
}
