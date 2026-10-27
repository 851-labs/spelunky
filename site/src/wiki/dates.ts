// A date as the wiki writes it, "27 October 2026": the verified line under a page's title (layout.tsx) and in
// its Markdown twin (build/markdown.ts). Written out by hand rather than with Intl, so that it's the same text
// on the server, in the browser and in the build, whatever locale data each has. No dependencies, since the
// pages' layout imports it as well as the build.
const MONTHS = ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"];

// An ISO 8601 date, YYYY-MM-DD.
export function longDate(date: string): string {
  const [year, month, day] = date.split("-").map(Number);
  return `${day} ${MONTHS[month - 1]} ${year}`;
}
