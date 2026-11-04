import { SiteFrame, SiteLinks, SiteSearch } from "./frame";

// The 404 page's content (851-4003), in the site's frame: what the 404 route pre-renders (src/routes/404.tsx), and
// what the router shows for a path it hasn't got (src/router.tsx: under `vite dev`, as src/server.ts serves it).
// Its styles are not-found.css (src/routes/__root.tsx links them).
export function NotFound() {
  return (
    <SiteFrame section={null} search={<SiteSearch />} footer={<SiteLinks />}>
      <h1>Nothing down here.</h1>
      <p>
        There's no page at this address. <a href="/">Back to the surface</a>, <a href="/play/">play</a>, or search the{" "}
        <a href="/wiki">wiki</a>.
      </p>
    </SiteFrame>
  );
}
