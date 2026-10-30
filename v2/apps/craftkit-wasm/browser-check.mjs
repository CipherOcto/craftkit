// The browser door, checked **without a browser**.
//
// A page is a program that runs in a browser, and a browser is a lot of
// machinery to ask for. But the module the page loads is a WebAssembly
// module, and Node runs those: so the whole boundary — the four
// exports, the two buffers, the command in and the answer out — can be
// driven from here, with no browser and no bundler.
//
//   cargo build -p craftkit-wasm --target wasm32-unknown-unknown --release
//   node v2/apps/craftkit-wasm/browser-check.mjs \
//       v2/target/wasm32-unknown-unknown/release/craftkit_wasm.wasm

import { readFile } from "node:fs/promises";

const path = process.argv[2];
if (!path) {
  console.error("the check needs the module's path: node browser-check.mjs <path to craftkit_wasm.wasm>");
  process.exit(1);
}

const bytes = await readFile(path);
const { instance } = await WebAssembly.instantiate(bytes, {});
const exports = instance.exports;

// The same two steps the page takes: write the command into the
// module's memory, call the exports, read the answer back out.
function ask(command) {
  const encoded = new TextEncoder().encode(command);
  const address = exports.craftkit_wasm_request_address(encoded.length);
  new Uint8Array(exports.memory.buffer).set(encoded, address);
  const length = exports.craftkit_wasm_run();
  const where_ = exports.craftkit_wasm_response_address();
  return new TextDecoder().decode(new Uint8Array(exports.memory.buffer, where_, length));
}

const commands = ask("commands").split("\n").filter((line) => line.length > 0);
console.log(`the kit holds ${commands.length} commands`);
console.log(commands.slice(0, 5).map((command) => `  ${command}`).join("\n"));

for (const question of ["session.memory", "doc.plan"]) {
  const answer = ask(question);
  console.log(`the answer to ${question}:`);
  console.log(answer.length > 0 ? answer : "  (nothing)");
}
