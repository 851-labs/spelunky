import { createRootRoute, HeadContent, Outlet, Scripts, useMatches } from "@tanstack/react-router";
import { useEffect } from "react";
import notFoundCss from "../site/not-found.css?url";
import { reloadOnSkew } from "../skew";
import { WikiStylesheet } from "../wiki/stylesheet";

export const Route = createRootRoute({
  head: () => ({
    meta: [{ charSet: "utf-8" }, { name: "viewport", content: "width=device-width, initial-scale=1" }],
  }),
  component: RootDocument,
});

function RootDocument() {
  useEffect(reloadOnSkew, []);
  // The 404 page has no scripts: it's served at whatever path wasn't found, where a router would hydrate another
  // page (src/routes/404.tsx). Its stylesheet is an element of the head while it's matched, not one of its head's
  // links, which React would keep as a resource after a client-side navigation away (as src/wiki/stylesheet.tsx's).
  const notFound = useMatches({ select: (matches) => matches.some((match) => match.routeId === "/404") });
  return (
    <html lang="en">
      <head>
        <HeadContent />
        <WikiStylesheet />
        {notFound && <link rel="stylesheet" href={notFoundCss} />}
      </head>
      <body>
        <Outlet />
        {!notFound && <Scripts />}
      </body>
    </html>
  );
}
