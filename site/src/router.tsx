import { createRouter } from "@tanstack/react-router";
import { routeTree } from "./routeTree.gen";

export function getRouter() {
  return createRouter({
    routeTree,
    // Only for a link the router follows itself: a request for a path it hasn't got gets public/404.html
    // (src/server.ts).
    defaultNotFoundComponent: () => <p>Nothing down here.</p>,
  });
}

declare module "@tanstack/react-router" {
  interface Register {
    router: ReturnType<typeof getRouter>;
  }
}
