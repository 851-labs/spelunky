import { createFileRoute } from "@tanstack/react-router";
import { pages } from "virtual:wiki/pages";
import { itemList } from "../../seo/jsonld";
import { seo } from "../../seo/seo";
import { WikiFrame } from "../../wiki/layout";
import css from "../../wiki/wiki.css?raw";

// /wiki: every page, by title, until 851-3336 makes it the hub, with the categories' hubs.
const sorted = [...pages].sort((a, b) => a.title.localeCompare(b.title, "en"));

export const Route = createFileRoute("/wiki/")({
  head: () => ({
    ...seo({
      title: "Spelunky Classic fan wiki",
      description:
        "An unofficial fan wiki for Spelunky Classic 1.1: its monsters, items, traps and places, with numbers " +
        "from the game's code and clips recorded from the game.",
      path: "/wiki",
      jsonLd: [
        itemList(
          "Spelunky Classic fan wiki pages",
          sorted.map((page) => ({ name: page.title, path: `/wiki/${page.slug}` })),
        ),
      ],
    }),
    styles: [{ children: css }],
  }),
  component: WikiIndex,
});

function WikiIndex() {
  return (
    <WikiFrame>
      <h1>Spelunky Classic fan wiki</h1>
      <p>
        An encyclopedia of Spelunky Classic 1.1, written from the game's own code. Every number cites the line it
        comes from, and every clip is recorded from the game playing.
      </p>
      <ul className="wiki-pages">
        {sorted.map((page) => (
          <li key={page.slug}>
            <a href={`/wiki/${page.slug}`}>{page.title}</a>: {page.lede}
          </li>
        ))}
      </ul>
    </WikiFrame>
  );
}
