import { createFileRoute, notFound } from "@tanstack/react-router";
import { HubView } from "../../../wiki/hub";
import { hubHead, loadHub } from "../../../wiki/hub-route";
import { WikiFrame } from "../../../wiki/layout";

// The items' categories' hubs: /wiki/items/equipment, /wiki/items/weapons, /wiki/items/carryables
// (src/wiki/hub.tsx). /wiki/items itself is the page route's, as the other top-level hubs are.
export const Route = createFileRoute("/wiki/items/$kind")({
  loader: async ({ params }) => {
    const hub = await loadHub(`items/${params.kind}`);
    if (!hub) throw notFound();
    return hub;
  },
  head: ({ loaderData }) => (loaderData ? hubHead(loaderData.hub, loaderData.data) : {}),
  component: ItemsHub,
});

function ItemsHub() {
  const { hub, data } = Route.useLoaderData();
  return (
    <WikiFrame>
      <HubView hub={hub} data={data} />
    </WikiFrame>
  );
}
