// A clip's anchor on its page (#clip-snake-bite for snake/bite), with no dependencies: the clip's figure has it
// (components.tsx), and References and the Markdown twins link to it.
export const clipAnchor = (id: string) => `clip-${id.replaceAll("/", "-")}`;
