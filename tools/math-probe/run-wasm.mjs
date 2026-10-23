// Runs the probe's wasm32 build under Node and prints its report.
// Usage: node run-wasm.mjs target/wasm32-unknown-unknown/release/math_probe.wasm
import { readFileSync } from "node:fs";

const { instance } = await WebAssembly.instantiate(readFileSync(process.argv[2]));
const { probe_run, probe_len, memory } = instance.exports;
const ptr = probe_run();
process.stdout.write(new TextDecoder().decode(new Uint8Array(memory.buffer, ptr, probe_len())));
