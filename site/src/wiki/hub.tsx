// The wiki's hubs: /wiki, and a category's (/wiki/monsters, /wiki/items/weapons), Fandom's category pages as
// a grid of sprite cards. A hub shows every page in its categories, written or not: the ones to come are
// dimmed cards rather than links, so the hub is the wiki's whole scope and what's left to write
// (src/wiki/build/hubs.ts). The grid filters as the reader types or picks an area; pre-rendered, it's all of
// it, and the filters work once the page has hydrated.
import { Field } from "@base-ui/react/field";
import { Input } from "@base-ui/react/input";
import { Toggle } from "@base-ui/react/toggle";
import { ToggleGroup } from "@base-ui/react/toggle-group";
import { useState } from "react";
import type { ReactNode } from "react";
import { CATEGORY_NAMES, childrenOf, HUBS } from "./build/categories.ts";
import type { Category, Hub } from "./build/categories.ts";
import type { HomeData, HubCard, HubData } from "./build/hubs";
import { hubTrail } from "./crumbs";
import { Breadcrumbs } from "./layout";

const plural = (n: number, one: string, many: string) => `${n} ${n === 1 ? one : many}`;

// How much of a list is written: "1 page written, 5 to come".
function Progress({ cards }: { cards: HubCard[] }) {
  const written = cards.filter((card) => card.href !== null).length;
  return (
    <p className="hub-count">
      {plural(written, "page", "pages")} written, {cards.length - written} to come.
    </p>
  );
}

export function HubView({ hub, data }: { hub: Hub; data: HubData }) {
  // /wiki/items's own: its categories' hubs, which it lists the cards of together.
  const children = childrenOf(hub.id);
  return (
    <article className="hub">
      <Breadcrumbs crumbs={hubTrail(hub)} />
      <h1>{hub.name}</h1>
      <p className="lede">{hub.description}</p>
      {children.length > 0 && (
        <nav className="hub-children" aria-label={`${hub.name} by kind`}>
          <ul>
            {children.map((child) => (
              <li key={child.id}>
                <a href={child.path}>{child.name}</a>
              </li>
            ))}
          </ul>
        </nav>
      )}
      {data.cards.length === 0 ? (
        <p className="hub-empty">No pages here yet: the wiki is new, and this category's are still to be written.</p>
      ) : (
        <>
          <Progress cards={data.cards} />
          <FilteredGrid hub={hub} data={data} />
        </>
      )}
    </article>
  );
}

type Status = "all" | "written" | "unwritten";

// The grid and its filters: a name, an area (where the hub's pages have areas), a kind (/wiki/items) and
// whether it's written yet (where some are and some aren't).
function FilteredGrid({ hub, data }: { hub: Hub; data: HubData }) {
  const [query, setQuery] = useState("");
  const [area, setArea] = useState<string | null>(null);
  const [kind, setKind] = useState<Category | null>(null);
  const [status, setStatus] = useState<Status>("all");
  const written = data.cards.filter((card) => card.href !== null).length;
  const mixed = written > 0 && written < data.cards.length;
  const words = query.trim().toLowerCase().split(/\s+/).filter(Boolean);
  const shown = data.cards.filter(
    (card) =>
      words.every((word) => card.title.toLowerCase().includes(word)) &&
      (area === null || card.areas.includes(area)) &&
      (kind === null || card.categories.includes(kind)) &&
      (status === "all" || (status === "written") === (card.href !== null)),
  );
  return (
    <>
      <div className="hub-filters" role="group" aria-label={`Filter the ${hub.name.toLowerCase()}`}>
        <Field.Root className="hub-query">
          <Field.Label>Filter by name</Field.Label>
          <Input type="search" value={query} onValueChange={setQuery} autoComplete="off" />
        </Field.Root>
        {data.areas.length > 0 && (
          <Choices
            label="Found in"
            value={area}
            onChange={setArea}
            choices={data.areas.map((a) => ({ value: a.slug, name: a.name }))}
          />
        )}
        {hub.categories.length > 1 && (
          <Choices
            label="Kind"
            value={kind}
            onChange={setKind}
            choices={hub.categories.map((c) => ({ value: c, name: CATEGORY_NAMES[c] }))}
          />
        )}
        {mixed && (
          <Choices
            label="Pages"
            value={status === "all" ? null : status}
            onChange={(value) => setStatus(value ?? "all")}
            choices={[
              { value: "written" as const, name: "Written" },
              { value: "unwritten" as const, name: "To come" },
            ]}
          />
        )}
      </div>
      <p className="hub-shown" aria-live="polite">
        {shown.length === data.cards.length ? `Showing all ${shown.length}.` : `Showing ${shown.length} of ${data.cards.length}.`}
      </p>
      {shown.length === 0 ? <p className="hub-empty">Nothing matches.</p> : <Grid cards={shown} />}
    </>
  );
}

// A row of toggles, one of which (or none, meaning all) is on: Base UI's toggle group, which the arrow keys
// move through. "All" is on when none of the others is, and pressing the one that's on turns it off.
function Choices<T extends string>({
  label,
  value,
  onChange,
  choices,
}: {
  label: string;
  value: T | null;
  onChange: (value: T | null) => void;
  choices: { value: T; name: string }[];
}) {
  return (
    <ToggleGroup
      className="hub-choices"
      aria-label={label}
      value={[value ?? ALL]}
      onValueChange={([pressed]) => onChange(pressed === undefined || pressed === ALL ? null : (pressed as T))}
    >
      <span className="hub-choices-label" aria-hidden="true">
        {label}
      </span>
      <Toggle value={ALL}>All</Toggle>
      {choices.map((choice) => (
        <Toggle key={choice.value} value={choice.value}>
          {choice.name}
        </Toggle>
      ))}
    </ToggleGroup>
  );
}
// "All"'s value, which no area, kind or status has.
const ALL = "*";

// The cards, with their ledes where they have them (/wiki's featured pages).
function Grid({ cards }: { cards: (HubCard & { lede?: string })[] }) {
  return (
    <ul className={cards.some((card) => card.lede) ? "cards cards-wide" : "cards"}>
      {cards.map((card) => (
        <li key={card.slug} className={card.href === null ? "card card-unwritten" : "card"}>
          <Card card={card} />
        </li>
      ))}
    </ul>
  );
}

// A card: the sprite (its name is the card's text, so the image is decoration) and the title. One to come is
// dimmed and says so, and isn't a link.
function Card({ card }: { card: HubCard & { lede?: string } }) {
  const inner = (
    <>
      <CardImage image={card.image} />
      <span className="card-title">{card.title}</span>
      {card.href === null && <span className="card-status">Not written yet</span>}
      {card.lede && <span className="card-lede">{card.lede}</span>}
    </>
  );
  return card.href === null ? <div className="card-body">{inner}</div> : <a className="card-body" href={card.href}>{inner}</a>;
}

// A sprite in its box, which has its size whether there's a sprite or not, so cards line up.
function CardImage({ image }: { image: HubCard["image"] }) {
  return (
    <span className="card-image">
      {image && <img src={image.src} width={image.width} height={image.height} alt="" loading="lazy" />}
    </span>
  );
}

// /wiki: the search box (the route's: src/wiki/search-box.tsx's WikiSearch, with /wiki?q='s results), the
// featured pages, every category, and how the wiki is made.
export function WikiHome({ data, search }: { data: HomeData; search: ReactNode }) {
  const counts = new Map(data.hubs.map((hub) => [hub.id, hub]));
  // Each top-level hub, with /wiki/items's own under it.
  const top = HUBS.filter((hub) => hub.parent === null);
  return (
    <article className="hub hub-home">
      <h1>Spelunky Classic fan wiki</h1>
      <p className="lede">
        An encyclopedia of Spelunky Classic 1.1, written from the game's own code. Every number cites the line it
        comes from, and every clip is recorded from the game playing.
      </p>
      {search}
      <p className="hub-count">
        {plural(data.written, "page", "pages")} written of the {data.total} the wiki covers so far.
      </p>

      {data.featured.length > 0 && (
        <section aria-labelledby="featured">
          <h2 id="featured">Featured pages</h2>
          <Grid cards={data.featured} />
        </section>
      )}

      <section aria-labelledby="categories">
        <h2 id="categories">Categories</h2>
        <ul className="categories">
          {top.map((hub) => {
            const count = counts.get(hub.id)!;
            const children = childrenOf(hub.id);
            return (
              <li key={hub.id} className="category">
                <a className="category-link" href={hub.path}>
                  <CardImage image={count.image} />
                  <span className="category-name">{hub.name}</span>
                  <span className="category-count">{count.total === 0 ? "No pages yet" : `${count.written} of ${count.total} written`}</span>
                </a>
                {children.length > 0 && (
                  <ul className="category-children">
                    {children.map((child) => (
                      <li key={child.id}>
                        <a href={child.path}>{child.name}</a>
                      </li>
                    ))}
                  </ul>
                )}
              </li>
            );
          })}
        </ul>
      </section>

      <section aria-labelledby="how" className="hub-how">
        <h2 id="how">How this wiki is made</h2>
        <p>
          The wiki is written from Spelunky Classic 1.1's GameMaker source, by Derek Yu, used under the{" "}
          <a href="/LICENSE.txt">Spelunky User License</a>. Its numbers (health, damage, speeds, prices) and
          relations (where things appear, what shops sell, what hurts what) are read from that code by the
          project's wiki-data tool, and each cites the lines it comes from.
        </p>
        <p>
          Its sprites are exported from the game's own assets, and its clips are recorded from the{" "}
          <a href="/play/">spelunky.ai port</a>, an unofficial Rust replica of the original that matches it tick
          for tick, bugs included, checked against the original game running under a reference oracle. Each clip
          is a scripted run of the real engine, so it can't show a move the game doesn't make. Every page says
          which release of the port it was last checked against; the code is on{" "}
          <a href="https://github.com/851-labs/spelunky">GitHub</a>.
        </p>
      </section>
    </article>
  );
}
