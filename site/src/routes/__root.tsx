import { createRootRoute, HeadContent, Outlet, Scripts, useMatches } from "@tanstack/react-router";
import { useEffect } from "react";
import downloadCss from "../site/download.css?url";
import notFoundCss from "../site/not-found.css?url";
import { reloadOnSkew } from "../skew";
import { WikiStylesheet } from "../wiki/stylesheet";

// The pages with a stylesheet of their own, by route: inlined from their heads, it would be in the route tree, which
// every page's entry chunk carries.
const PAGE_CSS: Partial<Record<string, string>> = { "/404": notFoundCss, "/download": downloadCss };

export const Route = createRootRoute({
  head: () => ({
    meta: [{ charSet: "utf-8" }, { name: "viewport", content: "width=device-width, initial-scale=1" }],
  }),
  component: RootDocument,
});

function RootDocument() {
  useEffect(reloadOnSkew, []);
  // The 404 page has no scripts: it's served at whatever path wasn't found, where a router would hydrate another
  // page (src/routes/404.tsx).
  const notFound = useMatches({ select: (matches) => matches.some((match) => match.routeId === "/404") });
  // A page's own stylesheet is an element of the head while it's matched, not one of its head's links, which React
  // would keep as a resource after a client-side navigation away (as src/wiki/stylesheet.tsx's).
  const pageCss = useMatches({
    select: (matches) => matches.map((match) => PAGE_CSS[match.routeId]).find((href) => href !== undefined),
  });
  return (
    <html lang="en">
      <head>
        <HeadContent />
        <WikiStylesheet />
        {pageCss && <link rel="stylesheet" href={pageCss} />}
      </head>
      <body>
        <Outlet />
        {!notFound && <Scripts />}
      </body>
    </html>
  );
}
