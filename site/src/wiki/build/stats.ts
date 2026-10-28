// The wiki's numbers, from the snapshot of scripts/wiki-data's stats.json committed under
// content/wiki-data/ (docs/wiki-data.md), as `<Stat>` and the infobox write them: "1 heart", "20–50 steps",
// "$15,000" (docs/wiki-style.md, "Numbers and units"). Resolved at build time, so the pages ship the text and
// never the data.
import { readFileSync } from "node:fs";

interface SnapshotStat {
  label: string;
  unit?: string;
  value?: number;
  min?: number;
  max?: number;
  // A table's rows (a price on each level): the variable's value, and the stat's there.
  table?: { at: number; value: number }[];
  at: string[];
}
interface SnapshotInit {
  // A string for a text constant (oSnake's `type = "Snake"`).
  value?: number | string;
  range?: { min: number; max: number; integer: boolean };
  // Whether the create event assigns it again under a condition, so the value isn't always this.
  changed_later?: boolean;
  at: string;
}
export interface Snapshot {
  format: number;
  room_speed: number;
  entities: Record<string, { name: string; kind: string; object?: string; stats: Record<string, SnapshotStat> }>;
  objects: Record<string, { parent?: string; type?: string; init: Record<string, SnapshotInit> }>;
}

export function readSnapshot(path: string): Snapshot {
  const data = JSON.parse(readFileSync(path, "utf8")) as Snapshot;
  if (data.format !== 1) throw new Error(`${path}: format ${data.format}, not 1: update site/src/wiki/build/stats.ts`);
  return data;
}

// How a value reads: `as="seconds"` turns steps into seconds and `as="per-second"` px/step into px/s;
// `bare` leaves the unit off. `at` reads a table's row instead (a price on level 5).
export interface StatOptions {
  as?: "seconds" | "per-second";
  bare?: boolean;
  at?: number;
}

export interface ResolvedStat {
  // What the page shows: "1 heart".
  text: string;
  // The number, for the <data> element: the value, or a range's "min-max".
  value: string;
  // Where it came from: gml/ paths and lines (objects/oSnake/create.gml:10).
  cite: string[];
}

// Each unit the specs use, singular and plural. A unit's text after a comma is kept as is ("px/step,
// plus the player's"). A unit this doesn't know fails the build, so a new one gets a wording.
const UNITS: Record<string, { one: string; many: string } | { prefix: string } | { template: string }> = {
  $: { prefix: "$" },
  hearts: { one: "heart", many: "hearts" },
  "hit points": { one: "hit point", many: "hit points" },
  steps: { one: "step", many: "steps" },
  px: { one: "pixel", many: "pixels" },
  "px/step": { one: "pixel per step", many: "pixels per step" },
  "per step": { one: "per step", many: "per step" },
  times: { one: "time", many: "times" },
  "1 in N per step": { template: "1 in N per step" },
};

// Thousands separators, and the decimals the code writes (the snapshot's numbers are the GML's literals).
export function formatNumber(n: number): string {
  if (!Number.isFinite(n)) throw new Error(`not a number: ${n}`);
  const [whole, fraction] = String(Math.abs(n)).split(".");
  const grouped = whole.replace(/\B(?=(\d{3})+(?!\d))/g, ",");
  return (n < 0 ? "-" : "") + grouped + (fraction ? `.${fraction}` : "");
}

// Two decimals at most (docs/wiki-style.md: `<Stat>`'s `as`).
const twoDecimals = (n: number) => Math.round(n * 100) / 100;

export function resolveStat(data: Snapshot, id: string, options: StatOptions = {}): ResolvedStat {
  const dot = id.indexOf(".");
  if (dot < 1) throw new Error(`stat "${id}" isn't <slug>.<stat> or <object>.<variable>`);
  const owner = id.slice(0, dot);
  const name = id.slice(dot + 1);
  let unit: string | undefined;
  let min: number;
  let max: number;
  let upTo = false;
  let cite: string[];
  if (/^o[A-Z]/.test(owner)) {
    if (options.at !== undefined) throw new Error(`stat "${id}": at= reads a table's row, and an object's create-event value has none`);
    // An object's create-event value (stats.json's `objects`): no unit, so it's shown bare.
    const object = data.objects[owner];
    if (!object) throw new Error(`stat "${id}": stats.json has no object ${owner}`);
    const init = object.init[name];
    if (!init) throw new Error(`stat "${id}": ${owner}'s create event sets no constant ${name}`);
    if (init.changed_later) {
      throw new Error(`stat "${id}": ${owner}'s create event changes ${name} again under a condition: cite its lines instead`);
    }
    if (typeof init.value === "string") throw new Error(`stat "${id}" is text (${JSON.stringify(init.value)}), not a number`);
    if (init.value !== undefined) min = max = init.value;
    else if (init.range) {
      ({ min, max } = init.range);
      // random(n): a real from 0 up to but not including n, "less than n".
      upTo = !init.range.integer && min === 0;
    } else throw new Error(`stat "${id}" has no value`);
    cite = [init.at];
  } else {
    const entity = data.entities[owner];
    if (!entity) throw new Error(`stat "${id}": no entity ${owner} in wiki/data/ (add a spec, docs/wiki-data.md)`);
    const stat = entity.stats[name];
    if (!stat) {
      throw new Error(`stat "${id}": ${owner} has no stat ${name} (it has ${Object.keys(entity.stats).join(", ")})`);
    }
    if (options.at !== undefined) {
      if (!stat.table) throw new Error(`stat "${id}" isn't a table, so it has no row at=${options.at}`);
      const row = stat.table.find((r) => r.at === options.at);
      if (!row) {
        const rows = stat.table.map((r) => r.at);
        throw new Error(`stat "${id}" has no row at=${options.at} (its rows are ${Math.min(...rows)} to ${Math.max(...rows)})`);
      }
      min = max = row.value;
    } else if (stat.value !== undefined) min = max = stat.value;
    else if (stat.min !== undefined && stat.max !== undefined) ({ min, max } = stat);
    else if (stat.table) throw new Error(`stat "${id}" is a table: read a row with at="..."`);
    else throw new Error(`stat "${id}" has no single value (an expression): cite its lines instead`);
    unit = stat.unit;
    cite = stat.at;
  }

  let [base, suffix] = unit === undefined ? [undefined, ""] : splitUnit(unit);
  if (options.as === "seconds") {
    if (base !== "steps") throw new Error(`stat "${id}": as="seconds" needs steps, not ${unit ?? "no unit"}`);
    [min, max] = [twoDecimals(min / data.room_speed), twoDecimals(max / data.room_speed)];
    base = "seconds";
  } else if (options.as === "per-second") {
    if (base !== "px/step") throw new Error(`stat "${id}": as="per-second" needs px/step, not ${unit ?? "no unit"}`);
    [min, max] = [twoDecimals(min * data.room_speed), twoDecimals(max * data.room_speed)];
    // "1 pixel per step (30 per second)" (docs/wiki-style.md, "Numbers and units").
    base = "per second";
  } else if (options.as !== undefined) {
    throw new Error(`stat "${id}": as="${options.as}" isn't seconds or per-second`);
  }

  const number = upTo ? `less than ${formatNumber(max)}` : min === max ? formatNumber(min) : `${formatNumber(min)}–${formatNumber(max)}`;
  const value = min === max ? String(min) : `${min}-${max}`;
  if (options.bare || base === undefined) return { text: number, value, cite };
  const words = base === "seconds" ? { one: "second", many: "seconds" } : base === "per second" ? { one: "per second", many: "per second" } : UNITS[base];
  if (!words) throw new Error(`stat "${id}": unit "${unit}" has no wording: add it to UNITS in site/src/wiki/build/stats.ts`);
  let text: string;
  if ("prefix" in words) text = words.prefix + number;
  else if ("template" in words) text = words.template.replace("N", number);
  else text = `${number} ${min === max && min === 1 ? words.one : words.many}`;
  return { text: text + suffix, value, cite };
}

function splitUnit(unit: string): [string, string] {
  const comma = unit.indexOf(",");
  return comma < 0 ? [unit, ""] : [unit.slice(0, comma), unit.slice(comma)];
}
