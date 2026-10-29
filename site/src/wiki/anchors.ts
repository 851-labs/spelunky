// A clip's anchor on its page (#clip-snake-bite for snake/bite), with no dependencies: the clip's figure has it
// (components.tsx), and References and the Markdown twins link to it.
export const clipAnchor = (id: string) => `clip-${id.replaceAll("/", "-")}`;

// The hub's "How this wiki is made" (hub.tsx's #how), which a quirk box's "How we know" links, on the page
// (layout.tsx) and in its twin.
export const HOW_WE_KNOW = "/wiki#how";
