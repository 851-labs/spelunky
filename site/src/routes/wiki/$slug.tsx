import { createFileRoute, notFound } from "@tanstack/react-router";
import { Suspense, use } from "react";
import { article, breadcrumbs, gameRef } from "../../seo/jsonld";
import { SITE_URL, seo } from "../../seo/seo";
import { WikiFrame, WikiPage } from "../../wiki/layout";
import { loadPage } from "../../wiki/pages";
import css from "../../wiki/wiki.css?raw";

// A wiki page, /wiki/<slug>: every page is pre-rendered (vite.config.ts lists them). Flat URLs whatever the
// page's type or categories (851-3329); the breadcrumbs, in JSON-LD, say where it sits. They go from /wiki
// straight to the page until the categories' hubs exist to put between them (851-3336).
export const Route = createFileRoute("/wiki/$slug")({
  loader: async ({ params }) => {
    const page = loadPage(params.slug);
    if (!page) throw notFound();
    const { meta } = await page;
    // What the head needs, which the browser gets with the page rather than loading the page's chunk.
    return {
      title: meta.title,
      lede: meta.lede,
      verified: meta.verified.date,
      image: meta.image?.src ?? null,
    };
  },
  head: ({ loaderData, params }) => {
    if (!loaderData) return {};
    const path = `/wiki/${params.slug}`;
    const { title, lede, verified, image } = loaderData;
    return {
      ...seo({
        title: `${title} – Spelunky Classic fan wiki`,
        description: lede,
        path,
        type: "article",
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
          breadcrumbs([
            { name: "Wiki", path: "/wiki" },
            { name: title, path },
          ]),
        ],
      }),
      styles: [{ children: css }],
    };
  },
  component: Page,
});

function Page() {
  const { slug } = Route.useParams();
  return (
    <WikiFrame>
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
