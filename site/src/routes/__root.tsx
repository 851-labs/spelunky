import { createRootRoute, HeadContent, Outlet, Scripts } from "@tanstack/react-router";
import { WikiStylesheet } from "../wiki/stylesheet";

export const Route = createRootRoute({
  head: () => ({
    meta: [{ charSet: "utf-8" }, { name: "viewport", content: "width=device-width, initial-scale=1" }],
  }),
  component: RootDocument,
});

function RootDocument() {
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
