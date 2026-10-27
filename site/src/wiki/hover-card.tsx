// Hover cards (docs/wiki-style.md, "Hover cards"): hovering a link to a written page, focusing it from the
// keyboard, or long-pressing it on a touch screen shows a card of the page's sprite, title, lede and first
// numbers, as Wikipedia's page previews do. A tap still follows the link.
//
// Base UI's preview card: every link to a written page (a page's body's, its relations', the navboxes') is a
// CardLink, a trigger of the one card the frame's HoverCards renders. A red link is a <span>, so it has none.
// Base UI opens the card on hover and keyboard focus, keeps it while the pointer is on it, closes it on
// Escape, and places it by the link (by the line the pointer's on, when the link wraps) inside the viewport,
// out of the page's flow, so nothing moves when it shows. The long press is ours: Base UI's card is for a
// mouse and a keyboard. The cards' data (build/cards.ts) is its own chunk, as is the popup's code
// (hover-card-popup.tsx), both loaded the first time a link is hovered or focused, so a page that's only
// read never loads them.
//
// The card is a tooltip too: role="tooltip", and the link's aria-describedby while it shows.
import { PreviewCard } from "@base-ui/react/preview-card";
import { useEffect } from "react";
import type { ReactNode } from "react";
import type { HoverCard } from "./build/cards";
import type { CardPopup, VirtualElement } from "./hover-card-popup";
import { loadOnce, never } from "./load-once";

// How long the pointer rests on a link before its card shows, how long the card stays once the pointer
// leaves (time to move onto it), and how long a touch is held to be a long press.
const SHOW_DELAY = 350;
const HIDE_DELAY = 250;
const LONG_PRESS = 500;
// How far a held touch may move and still be a press rather than a scroll.
const PRESS_SLOP = 10;
// There's one card on a page, the frame's.
const CARD_ID = "hover-card";

interface Loaded {
  cards: Record<string, HoverCard>;
  CardPopup: typeof CardPopup;
}

// The cards and their popup, once they're loaded.
let loaded: Loaded | null = null;

// The cards and their popup, from their own chunks. The Worker never shows one, so its bundle needn't carry
// them.
const loadCards = loadOnce(() =>
  Promise.all([
    import.meta.env.SSR ? never() : import("virtual:wiki/hover-cards"),
    import.meta.env.SSR ? never() : import("./hover-card-popup"),
  ]).then(([{ cards }, { CardPopup }]) => (loaded = { cards, CardPopup })),
);
// Started as the mouse reaches a link, so the cards are there by the time its card shows. Not for a touch,
// whose tap leaves the page.
const preload = () => void loadCards().catch(() => {});
// A link's card, at once (once it's loaded) if the link still wants it.
const show = (link: HTMLAnchorElement) => void loadCards().then(() => wanted(link) && handle.open(link.id), () => {});

// The links' card, which a link's slug (its trigger's payload) picks.
const handle = PreviewCard.createHandle<string>();

// A link to a written page, with its card.
export function CardLink({ slug, href, className, children }: { slug: string; href: string; className?: string; children?: ReactNode }) {
  return (
    <PreviewCard.Trigger
      handle={handle}
      payload={slug}
      href={href}
      className={className}
      data-slug={slug}
      delay={SHOW_DELAY}
      closeDelay={HIDE_DELAY}
      onPointerEnter={(e) => e.pointerType !== "touch" && preload()}
      // From the keyboard, at once, rather than after the hover's delay (a click focuses it too, but the
      // pointer's on it already).
      onFocus={(e) => e.currentTarget.matches(":focus-visible") && show(e.currentTarget)}
      // Described by the card while it shows.
      render={(props, state) => <a {...props} aria-describedby={state.open ? CARD_ID : undefined} />}
    >
      {children}
    </PreviewCard.Trigger>
  );
}

const linkOf = (target: EventTarget | null) => (target instanceof Element ? target.closest<HTMLAnchorElement>("a[data-slug]") : null);

// A link that's been long-pressed, whose click (if the browser sends one when the finger lifts) mustn't
// follow it.
let pressed: HTMLAnchorElement | null = null;
// The link whose long press asked for its card, until the card closes, and the line the finger was on (a
// wrapped link's), which the card goes by: Base UI only knows a mouse's.
let asked: { link: HTMLAnchorElement; line: VirtualElement } | null = null;

// The card's link still wants it: the pointer's on it, it has the keyboard's focus, or it's been pressed.
const wanted = (link: HTMLAnchorElement) => link.isConnected && (link === asked?.link || link.matches(":hover, :focus-visible"));

// The line of the link at (x, y), for the card to be placed by.
function lineAt(link: HTMLAnchorElement, x: number, y: number): VirtualElement {
  const index = Math.max(0, [...link.getClientRects()].findIndex((r) => x >= r.left && x <= r.right && y >= r.top && y <= r.bottom));
  return { getBoundingClientRect: () => link.getClientRects()[index] ?? link.getBoundingClientRect(), contextElement: link };
}

export function HoverCards() {
  // The long press: a touch held on a link, without moving, opens its card. The card closes as any does,
  // when the reader taps elsewhere.
  useEffect(() => {
    let press: { link: HTMLAnchorElement; x: number; y: number; timer: ReturnType<typeof setTimeout> } | null = null;
    const endPress = () => {
      if (press) clearTimeout(press.timer);
      press = null;
    };
    const open = (link: HTMLAnchorElement, x: number, y: number) => {
      endPress();
      pressed = link;
      asked = { link, line: lineAt(link, x, y) };
      show(link);
    };
    const listeners: { [K in keyof DocumentEventMap]?: (e: DocumentEventMap[K]) => void } = {
      pointerdown(e) {
        pressed = null;
        endPress();
        const link = linkOf(e.target);
        if (link && e.pointerType === "touch") {
          const { clientX: x, clientY: y } = e;
          press = { link, x, y, timer: setTimeout(() => open(link, x, y), LONG_PRESS) };
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
        if (link !== pressed) open(link, press!.x, press!.y);
      },
      click(e) {
        if (pressed && linkOf(e.target) === pressed) e.preventDefault();
        pressed = null;
      },
    };
    const all = Object.entries(listeners) as [string, EventListener][];
    for (const [type, listener] of all) document.addEventListener(type, listener);
    // Not left open for the page's return from the back/forward cache, after a tap on its link.
    const left = () => handle.close();
    window.addEventListener("pagehide", left);
    return () => {
      for (const [type, listener] of all) document.removeEventListener(type, listener);
      window.removeEventListener("pagehide", left);
      endPress();
    };
  }, []);

  return (
    <PreviewCard.Root
      handle={handle}
      onOpenChange={(open, details) => {
        // Not until the link's card is loaded: then, if the link still wants it. A link with no card (the
        // chunk failed to load, say) keeps working, and the next hover tries again.
        const link = details.trigger;
        if (!open) asked = null;
        if (!open || !(link instanceof HTMLAnchorElement)) return;
        if (loaded && Object.hasOwn(loaded.cards, link.dataset.slug!)) return;
        details.cancel();
        if (!loaded) loadCards().then(() => wanted(link) && handle.open(link.id), () => {});
      }}
    >
      {({ payload }) => {
        const card = payload === undefined ? undefined : loaded?.cards[payload];
        const line = asked && asked.link.dataset.slug === payload ? asked.line : undefined;
        return card && loaded && <loaded.CardPopup id={CARD_ID} card={card} anchor={line} />;
      }}
    </PreviewCard.Root>
  );
}
