import { createRootRoute, HeadContent, Outlet, Scripts } from "@tanstack/react-router";
import { useEffect } from "react";
import { buildMeta, reloadOnSkew } from "../skew";
import { WikiStylesheet } from "../wiki/stylesheet";

export const Route = createRootRoute({
  head: () => ({
    meta: [{ charSet: "utf-8" }, { name: "viewport", content: "width=device-width, initial-scale=1" }, ...buildMeta()],
  }),
  component: RootDocument,
});

function RootDocument() {
  useEffect(reloadOnSkew, []);
  return (
    <html lang="en">
      <head>
        <HeadContent />
        <WikiStylesheet />
      </head>
      <body>
        <Outlet />
        <Scripts />
      </body>
    </html>
  );
}
