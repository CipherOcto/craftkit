# Attribution

Every file in this repository that is not source code — and every piece of
source code adapted from another project — listed here, with what it is and
where it came from.

`cargo xtask assets` enforces this. It fails when a file asset appears with no
row here, and when a row names a file that is not in the tree. Adding an asset
means adding a row in the same change; removing one means removing the row.

## Files

**There are no images, fonts, audio, video or model files.** This repository
contains no asset of any kind: no PNG, no SVG, no icon font, no sprite sheet,
no typeface file, no sample, no model.

That is not an accident and not a temporary state, so it is worth saying why,
because "we have no assets yet" and "we have no assets on purpose" are very
different positions:

- **Nothing is drawn from a file.** A command's `icon` is a **name**, not a
  picture: the front end decides what to draw, and the tree's own front end
  (`apps/craftkit-window`) draws every command as a text button. There is no
  image to embed, so there is no image whose provenance is in question.
- **The typeface is arithmetic, not artwork.** `crates/craftkit-font` holds
  glyphs as strokes in code, in the crate's own coordinate system. A new
  glyph is a new set of coordinates, not a new file.
- **Provenance has to be provable.** An icon that arrives as a PNG arrives with
  no record of who drew it. An icon that arrives as coordinates arrives with
  its history in the commit that added it. A gate that keeps the first out of
  the repository is cheaper than any argument about the second.

`cargo xtask assets` checks the mechanical part of this: no `include_bytes!`,
`include_str!` or `include!` of anything that is not source code, anywhere in
the crates. The three `include!(concat!(env!("OUT_DIR"), "/plugins.rs"))` calls
in `apps/craftkit-bridge`, `apps/craftkit-studio` and `crates/craftkit-link`
are **generated Rust**, not assets: they embed the linker output, which is
built from this tree's own crates.

## The files that are not Rust

Three files in this repository are not Rust source. Each one is here because
the platform it serves insists on that language, and none of them runs
anything of the tree's own logic.

| File | What | Source |
|---|---|---|
| `apps/craftkit-wasm/index.html` | the page the browser door serves; it loads the WebAssembly module and drives it | original |
| `apps/craftkit-wasm/browser-check.mjs` | the Node script that drives the same WebAssembly module without a browser | original |
| `.github/workflows/ci.yml` | the continuous-integration workflow | original |

The rule is the same one the tree follows everywhere: **Rust only**, and the
exceptions are named here rather than allowed by default. The gate that
enforces it is `cargo xtask assets`; the table of exceptions is
`FOREIGN_SOURCE_EXCEPTIONS` in `xtask/src/assets.rs`, and adding a row there
means adding a row here in the same change.

## Text

| File | What | Source |
|---|---|---|
| `README.md`, `AGENTS.md`, `CONTRIBUTING.md`, `KNOWN-GAPS.md`, `docs/**`, `skills/**` | The project's own documentation | Written for this project. No third-party text. |
| `LICENSE` | The dual-licence notice | Written for this project. It names the choice (MIT OR Apache-2.0) and points at the canonical texts rather than transcribing them — a paraphrase of a licence is not a licence. |

## Third-party code

**None is vendored.** There is no `vendor/` directory, no patched dependency
and no copied source.

### Dependencies

The build depends on the crates in `Cargo.toml`, each under its own licence,
and ships none of their source.

| Crate | Licence | Used for |
|---|---|---|
| `serde`, `serde_json` | MIT OR Apache-2.0 | the saved format, the control channel, MCP |
| `inventory` | MIT OR Apache-2.0 | the registry: a crate that is linked in contributes its commands |
| `egui`, `eframe` | MIT OR Apache-2.0 | the window |
| `kurbo` | MIT OR Apache-2.0 | the bezier arithmetic the batteries' path operations use |
| `linesweeper` | MIT OR Apache-2.0 | the robust sweep-line boolean operation |
| `syn`, `quote`, `proc-macro2`, `proc-macro-crate` | MIT OR Apache-2.0 | `craftkit-derive`, which writes the declaration, the reader and the registration |

The policy that keeps this list honest is `deny.toml`, checked by
`cargo xtask deny` on every push: every dependency's licence must be on the
allowed list, every wildcard must be denied, every source must be known, and
every advisory must be answered.

### Adapted code

Code that was taken from another project and reshaped is listed here with its
upstream and licence, even where the result has been rewritten. If nothing is
listed, nothing was adapted — which is the case today.

### Clean-room statement

Nothing in this repository was derived from the source of another CAD,
typesetting, audio or 3D application. Behaviour was designed from published
specifications, general conventions and first principles. The rule and the
table of products it names are in `docs/clean-room.md`, and `cargo xtask
cleanroom` checks the tree against that table.

## Adding a row

A row is a table row with three things in it: the file, what it is, and where
it came from. "Where it came from" is the part that matters. If you cannot
fill in that column, the file should not be in the repository.

For a file you drew yourself, that is enough:

```
| `apps/craftkit-wasm/index.html` | the page the browser door serves | original |
```

For anything from elsewhere, say which project, which licence, and where in
it the file came from. If the licence is not one this project can ship, the
answer is not to add a row — it is to leave the file out.
