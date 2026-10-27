import { createRouter } from "@tanstack/react-router";
import { routeTree } from "./routeTree.gen";

export function getRouter() {
  return createRouter({
    routeTree,
    // Only for a link the router follows itself: a request for a path it hasn't got gets public/404.html
    // (src/server.ts).
    defaultNotFoundComponent: () => <p>Nothing down here.</p>,
    // Plain query strings, each value a string: /wiki?q=30 searches for "30", where the router's default
    // would read the number 30 and write the string back as q=%2230%22. The WebSite JSON-LD's SearchAction
    // fills in /wiki?q={search_term_string} plainly too (src/seo/jsonld.ts).
    parseSearch: (search) => Object.fromEntries(new URLSearchParams(search)),
    stringifySearch: (search) => {
      const params = new URLSearchParams();
      for (const [key, value] of Object.entries(search)) if (value !== undefined) params.set(key, String(value));
      const query = params.toString();
      return query ? `?${query}` : "";
    },
  });
}

declare module "@tanstack/react-router" {
  interface Register {
    router: ReturnType<typeof getRouter>;
  }
}
