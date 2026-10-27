import { createFileRoute, notFound } from "@tanstack/react-router";
import { Suspense, use } from "react";
import { article, breadcrumbs, gameRef } from "../../seo/jsonld";
import { SITE_URL, seo } from "../../seo/seo";
import { trail } from "../../wiki/crumbs";
import { HubView } from "../../wiki/hub";
import { hubHead, isTopHub, loadHub } from "../../wiki/hub-route";
import { WikiFrame, WikiPage } from "../../wiki/layout";
import { loadPage } from "../../wiki/pages";
import css from "../../wiki/wiki.css?raw";

// A wiki page, /wiki/<slug>: every page is pre-rendered (vite.config.ts lists them). Flat URLs whatever the
// page's type or categories (851-3329); the breadcrumbs, on the page and in JSON-LD, say where it sits
// (src/wiki/crumbs.ts). A top-level hub's path (/wiki/monsters) is one segment too, and no page can take its
// slug (schema.ts's RESERVED_SLUGS), so the hubs are here as well (src/wiki/hub.tsx).
export const Route = createFileRoute("/wiki/$slug")({
  loader: async ({ params }) => {
    if (isTopHub(params.slug)) {
      const hub = await loadHub(params.slug);
      if (!hub) throw notFound();
      return { hub };
    }
    const page = loadPage(params.slug);
    if (!page) throw notFound();
    const { meta } = await page;
    // What the head needs, which the browser gets with the page rather than loading the page's chunk.
    return {
      page: {
        title: meta.title,
        lede: meta.lede,
        verified: meta.verified.date,
        image: meta.image?.src ?? null,
        crumbs: trail(meta),
        // Its FAQPage, its clips' VideoObjects and its sprite's ImageObject, made at build time (compile.ts):
        // this code is the browser's too, which mustn't get the whole media index.
        jsonLd: meta.jsonLd,
        // For the footer's audio credit, which is outside the page's chunk.
        audio: meta.audio,
        og: meta.og,
      },
    };
  },
  head: ({ loaderData, params }) => {
    if (loaderData?.hub) return hubHead(loaderData.hub.hub, loaderData.hub.data);
    if (!loaderData?.page) return {};
    const path = `/wiki/${params.slug}`;
    const { title, lede, verified, image, crumbs, jsonLd, og } = loaderData.page;
    return {
      ...seo({
        title: `${title} – Spelunky Classic fan wiki`,
        description: lede,
        path,
        type: "article",
        image: og ?? undefined,
        jsonLd: [
          article({
            headline: title,
            description: lede,
            path,
            about: [{ "@type": "Thing", name: title }, gameRef()],
            // The media's paths are escaped already (%40), which absoluteUrl won't take as a site path.
            image: image === null ? undefined : SITE_URL + image,
            dateModified: verified,
          }),
          breadcrumbs(crumbs),
          ...jsonLd,
        ],
        // Its Markdown twin (src/wiki/build/markdown.ts).
        alternates: [{ type: "text/markdown", path: `${path}.md` }],
      }),
      styles: [{ children: css }],
    };
  },
  component: Page,
});

function Page() {
  const { slug } = Route.useParams();
  const { hub, page } = Route.useLoaderData();
  if (hub) {
    return (
      <WikiFrame>
        <HubView hub={hub.hub} data={hub.data} />
      </WikiFrame>
    );
  }
  return (
    <WikiFrame audio={page?.audio}>
      {/* Only the browser ever waits here, hydrating before the page's chunk has loaded: it keeps the
          pre-rendered page meanwhile (src/wiki/pages.ts). */}
      <Suspense fallback={null}>
        <Loaded slug={slug} />
      </Suspense>
    </WikiFrame>
  );
}

function Loaded({ slug }: { slug: string }) {
  const page = loadPage(slug);
  if (!page) throw notFound();
  return <WikiPage page={use(page)} />;
}
