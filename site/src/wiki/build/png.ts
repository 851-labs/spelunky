// Just enough PNG for the Open Graph cards (og.ts): reading the wiki media's sprites and writing the cards,
// with node:zlib and nothing else, so the site's build needs no native or wasm image library (Workers Builds
// has Node, and npm installs no install scripts for it: 851-3391).
import { crc32, deflateSync, inflateSync } from "node:zlib";

// 8-bit RGBA, row by row.
export interface Rgba {
  width: number;
  height: number;
  data: Uint8Array;
}

const SIGNATURE = Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]);

// A PNG's image (an APNG's default image, its first frame for the wiki's), as RGBA. Takes the formats a PNG
// encoder writes for pixel art (8-bit grey, RGB, palette, grey with alpha, RGBA; a palette of 1, 2 or 4 bits
// too), not interlaced; throws on anything else, rather than guessing.
export function decodePng(file: Uint8Array, what: string): Rgba {
  const bytes = Buffer.from(file.buffer, file.byteOffset, file.byteLength);
  if (!bytes.subarray(0, 8).equals(SIGNATURE)) throw new Error(`${what}: not a PNG`);
  let header: { width: number; height: number; depth: number; type: number } | null = null;
  let palette: Buffer | null = null;
  let transparency: Buffer | null = null;
  const idat: Buffer[] = [];
  for (let at = 8; at + 12 <= bytes.length; ) {
    const length = bytes.readUInt32BE(at);
    const type = bytes.toString("latin1", at + 4, at + 8);
    const body = bytes.subarray(at + 8, at + 8 + length);
    at += 12 + length;
    if (type === "IHDR") {
      if (body[10] !== 0 || body[11] !== 0) throw new Error(`${what}: an unknown compression or filter method`);
      if (body[12] !== 0) throw new Error(`${what}: interlaced, which og.ts doesn't read`);
      header = { width: body.readUInt32BE(0), height: body.readUInt32BE(4), depth: body[8], type: body[9] };
    } else if (type === "PLTE") palette = body;
    else if (type === "tRNS") transparency = body;
    else if (type === "IDAT") idat.push(body);
    else if (type === "IEND") break;
  }
  if (!header) throw new Error(`${what}: no IHDR`);
  const { width, height, depth, type } = header;
  const channels = { 0: 1, 2: 3, 3: 1, 4: 2, 6: 4 }[type];
  if (channels === undefined || (depth !== 8 && !(type === 3 && [1, 2, 4].includes(depth)))) {
    throw new Error(`${what}: colour type ${type} at ${depth} bits, which og.ts doesn't read`);
  }
  if (type === 3 && !palette) throw new Error(`${what}: a palette image without a PLTE`);
  const raw = inflateSync(Buffer.concat(idat));
  const stride = Math.ceil((width * channels * depth) / 8);
  if (raw.length < (stride + 1) * height) throw new Error(`${what}: too little image data`);
  // Unfiltered rows; `bpp` is the filters' byte distance, at least 1.
  const bpp = Math.max(1, (channels * depth) / 8);
  const rows = new Uint8Array(stride * height);
  for (let y = 0; y < height; y++) {
    const filter = raw[y * (stride + 1)];
    const line = raw.subarray(y * (stride + 1) + 1, (y + 1) * (stride + 1));
    const out = y * stride;
    for (let x = 0; x < stride; x++) {
      const a = x >= bpp ? rows[out + x - bpp] : 0;
      const b = y > 0 ? rows[out - stride + x] : 0;
      const c = x >= bpp && y > 0 ? rows[out - stride + x - bpp] : 0;
      let predicted: number;
      if (filter === 0) predicted = 0;
      else if (filter === 1) predicted = a;
      else if (filter === 2) predicted = b;
      else if (filter === 3) predicted = (a + b) >> 1;
      else if (filter === 4) {
        const p = a + b - c;
        const pa = Math.abs(p - a);
        const pb = Math.abs(p - b);
        const pc = Math.abs(p - c);
        predicted = pa <= pb && pa <= pc ? a : pb <= pc ? b : c;
      } else throw new Error(`${what}: unknown filter ${filter}`);
      rows[out + x] = (line[x] + predicted) & 0xff;
    }
  }
  const data = new Uint8Array(width * height * 4);
  for (let y = 0; y < height; y++) {
    for (let x = 0; x < width; x++) {
      const o = (y * width + x) * 4;
      const row = y * stride;
      if (type === 3) {
        const bit = x * depth;
        const index = (rows[row + (bit >> 3)] >> (8 - depth - (bit & 7))) & ((1 << depth) - 1);
        if (index * 3 + 2 >= palette!.length) throw new Error(`${what}: a pixel past the palette`);
        data.set(palette!.subarray(index * 3, index * 3 + 3), o);
        data[o + 3] = transparency && index < transparency.length ? transparency[index] : 255;
        continue;
      }
      const p = row + x * channels;
      if (type === 0 || type === 4) {
        data[o] = data[o + 1] = data[o + 2] = rows[p];
        data[o + 3] = type === 4 ? rows[p + 1] : 255;
      } else {
        data[o] = rows[p];
        data[o + 1] = rows[p + 1];
        data[o + 2] = rows[p + 2];
        data[o + 3] = type === 6 ? rows[p + 3] : 255;
      }
    }
  }
  return { width, height, data };
}

// An opaque RGB image as a PNG: indexed, its palette in the order the colours first appear, when it has 256
// colours or fewer (a card has a few dozen), else RGB. Unfiltered, as pixel art's flat runs compress best
// that way, then deflated at level 9. `text` adds tEXt chunks (Latin-1), as the media's sprites carry their
// notice in `Copyright`. The same pixels always give the same file on the same zlib.
export function encodePng(width: number, height: number, rgb: Uint8Array, text: Record<string, string> = {}): Buffer {
  const colours = new Map<number, number>();
  for (let i = 0; i < rgb.length && colours.size <= 256; i += 3) {
    const key = (rgb[i] << 16) | (rgb[i + 1] << 8) | rgb[i + 2];
    if (!colours.has(key)) colours.set(key, colours.size);
  }
  const indexed = colours.size <= 256;
  const stride = indexed ? width : width * 3;
  const raw = Buffer.alloc((stride + 1) * height);
  for (let y = 0; y < height; y++) {
    const out = y * (stride + 1) + 1;
    if (!indexed) {
      raw.set(rgb.subarray(y * stride, (y + 1) * stride), out);
      continue;
    }
    for (let x = 0; x < width; x++) {
      const i = (y * width + x) * 3;
      raw[out + x] = colours.get((rgb[i] << 16) | (rgb[i + 1] << 8) | rgb[i + 2])!;
    }
  }
  const ihdr = Buffer.alloc(13);
  ihdr.writeUInt32BE(width, 0);
  ihdr.writeUInt32BE(height, 4);
  ihdr[8] = 8;
  ihdr[9] = indexed ? 3 : 2;
  const chunks = [chunk("IHDR", ihdr)];
  if (indexed) {
    const plte = Buffer.alloc(colours.size * 3);
    for (const [key, index] of colours) plte.set([key >> 16, (key >> 8) & 0xff, key & 0xff], index * 3);
    chunks.push(chunk("PLTE", plte));
  }
  for (const [key, value] of Object.entries(text)) chunks.push(chunk("tEXt", Buffer.from(`${key}\0${value}`, "latin1")));
  chunks.push(chunk("IDAT", deflateSync(raw, { level: 9 })), chunk("IEND", Buffer.alloc(0)));
  return Buffer.concat([SIGNATURE, ...chunks]);
}

function chunk(type: string, body: Buffer): Buffer {
  const out = Buffer.alloc(12 + body.length);
  out.writeUInt32BE(body.length, 0);
  out.write(type, 4, "latin1");
  body.copy(out, 8);
  out.writeUInt32BE(crc32(out.subarray(4, 8 + body.length)), 8 + body.length);
  return out;
}
