// A hover card's popup (src/wiki/hover-card.tsx): Base UI's positioner and popup, and the card in it. Its own
// chunk, loaded with the cards' data the first time a link is hovered or focused, so the pages' bundle
// carries only the links' triggers.
import { PreviewCard } from "@base-ui/react/preview-card";
import type { RefObject } from "react";
import type { HoverCard } from "./build/cards";
import { Sprite } from "./components";

// The card's distance from the link, and its least from the viewport's edges: past its stone frame's two
// rings and shadow (wiki.css's --stone), 8 and 16 pixels.
const GAP = 12;
const MARGIN = 20;

// Placed under the link, its left edge at the link's; over it when only that fits; where neither does, as
// low as the viewport lets it, over the link (`sticky` shifts it up and down, and no other side is tried, as
// beside the link it would cover the line). Inside the viewport (its CSS keeps it shorter), so it never
// lengthens the page.
const AVOID_COLLISIONS = { fallbackAxisSide: "none" } as const;

// What a card is placed by, when it isn't its link: the line of it a long press was on.
export type VirtualElement = Exclude<PreviewCard.Positioner.Props["anchor"], Element | RefObject<Element | null> | (() => unknown) | null | undefined>;

export function CardPopup({ id, card, anchor }: { id: string; card: HoverCard; anchor?: VirtualElement }) {
  return (
    <PreviewCard.Portal>
      <PreviewCard.Positioner
        className="hover-card-positioner"
        anchor={anchor}
        side="bottom"
        align="start"
        sideOffset={GAP}
        collisionPadding={MARGIN}
        collisionAvoidance={AVOID_COLLISIONS}
        sticky
      >
        <PreviewCard.Popup id={id} role="tooltip" className="hover-card">
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
        </PreviewCard.Popup>
      </PreviewCard.Positioner>
    </PreviewCard.Portal>
  );
}
