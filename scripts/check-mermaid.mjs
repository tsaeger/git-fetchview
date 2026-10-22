// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2026 Tom Saeger <tom.saeger@gmail.com>

// Parse every ```mermaid block in the given Markdown files. Exits 1 if any
// block does not parse. Needs no browser: mermaid only parses here, it does
// not render, so a jsdom window is enough.
import { readFileSync } from "node:fs";
import { JSDOM } from "jsdom";

const dom = new JSDOM("<!doctype html><body></body>");
globalThis.window = dom.window;
globalThis.document = dom.window.document;
const { default: mermaid } = await import("mermaid");

let failed = 0;
let blocks = 0;
for (const file of process.argv.slice(2)) {
  const text = readFileSync(file, "utf8");
  for (const m of text.matchAll(/^```mermaid\n([\s\S]*?)^```/gm)) {
    blocks += 1;
    const line = text.slice(0, m.index).split("\n").length;
    try {
      await mermaid.parse(m[1]);
    } catch (e) {
      failed += 1;
      console.error(`${file}:${line}: ${String(e.message ?? e).split("\n")[0]}`);
    }
  }
}
console.log(`mermaid: ${blocks} block(s), ${failed} failed`);
process.exit(failed === 0 ? 0 : 1);
