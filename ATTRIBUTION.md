# Attribution

Every file in this repository that is not source code — and every piece of
source code adapted from another project — listed here, with what it is and
where it came from.

`cargo xtask assets` enforces this. It fails when a file asset appears with no
row here, and when a row names a file that is not in the tree. Adding an asset
means adding a row in the same change; removing one means removing the row.

## Files

**There are none.** This repository contains no image, font, audio, video or
model file of any kind.

That is not an accident and not a temporary state, so it is worth saying why,
because "we have no assets yet" and "we have no assets on purpose" are very
different positions:

- **The UI icons are drawn in code.** `crates/ui/src/icons.rs` holds each icon
  as a set of line segments and arcs in the crate's own coordinate system, and
  the shell strokes them. There is no icon font, no sprite sheet, no SVG. A new
  icon is a new `Icon` constant, not a new file.
- **The type is a parameter, not a picture.** craftkit's glyphs are numbers, not
  artwork. Nothing in a design file is a picture, so nothing that draws one is
  embedded.
- **Provenance has to be provable.** An icon that arrives as a PNG arrives with
  no record of who drew it. An icon that arrives as coordinates arrives with its
  history in the commit that added it. A gate that keeps the first out of the
  repository is cheaper than any argument about the second.

`cargo xtask assets` checks the mechanical part of this: no `include_bytes!`,
`include_str!` or `include!` of anything that is not source code, anywhere in
the crates. See `xtask/src/assets.rs`.

## Text

| File | What | Source |
|---|---|---|
| `README.md`, `AGENTS.md`, `CONTRIBUTING.md`, `docs/**` | The project's own documentation | Written for this project. No third-party text. |
| `LICENSE` | The dual-licence notice | Written for this project. It names the choice (MIT OR Apache-2.0) and points at the canonical texts rather than transcribing them — a paraphrase of a licence is not a licence. |

## Third-party code

**None is vendored.** There is no `vendor/` directory, no patched dependency and
no copied source.

### Dependencies

The build depends on the crates in `Cargo.toml`, each under its own licence, and
ships none of their source. Notably:

| Crate | Licence | Used for |
|---|---|---|
| `serde`, `serde_json` | MIT OR Apache-2.0 | the saved format, the control channel, MCP |
| `thiserror` | MIT OR Apache-2.0 | the error types |
| `egui`, `eframe`, `epaint` | MIT OR Apache-2.0 | the shell and the window |
| `image` | MIT OR Apache-2.0 | PNG encoding for `craftkit-cli snapshot` |
| `log` | MIT OR Apache-2.0 | the logging facade |
| `wasm-bindgen`, `web-sys`, `js-sys` | MIT OR Apache-2.0 | the browser build only |

`cargo deny` (configuration in `deny.toml`) is the tool that keeps this list
honest; `THIRD-PARTY-LICENSES.txt` is generated from it and is not edited by
hand.

### Adapted code

Code that was taken from another project and reshaped is listed here with its
upstream and licence, even where the result has been rewritten. If nothing is
listed, nothing was adapted — which is the case today.

### Clean-room statement

Nothing in this repository was derived from the source of another CAD,
typesetting or 3D application. Behaviour was designed from published
specifications, general conventions and first principles. Where a format is
specified by somebody else — and a file format is — the specification is cited
in `crates/io/src/format.rs`.

## Adding a row

A row is a table row with three things in it: the file, what it is, and where it
came from. "Where it came from" is the part that matters. If you cannot fill in
that column, the file should not be in the repository.

For a file you drew yourself, that is enough:

```
| `assets/icons/source/wrench.svg` | the wrench icon, first draft | original |
```

For anything from elsewhere, say which project, which licence, and where in it
the file came from. If the licence is not one this project can ship, the answer
is not to add a row — it is to leave the file out.
