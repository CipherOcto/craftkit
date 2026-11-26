# The craft-libs plan: what the upstream has and this tree does not

The upstream is a large collection of format crates; this tree reads and writes a
small fraction of those formats today. This plan names the capabilities worth
porting, the arcs that would port them, and the order the arcs would come in.

Nothing here is built. Every arc below is future work, and every capability
named as absent is absent as of this writing.

## 0. The rule

Every arc serves one rule: **a file format is read and written by one shared
library, and the batteries call it.**

## 1. What the survey found

| capability | upstream path:line | craftkit's state | verdict |
|---|---|---|---|
| TIFF / BigTIFF read and write (IFD chain, Exif/GPS pointers, strips and tiles) | `crates/tiff/Cargo.toml:3` | absent | PORT |
| WebP read and write (canvas, alpha, animation frames) | `crates/webp/Cargo.toml:3` | absent | PORT |
| 3-D mesh containers: STL, OBJ, glTF/GLB, 3MF, PLY, FBX behind one neutral model | `crates/mesh/Cargo.toml:3` | absent — craftkit has no mesh format at all | PORT |
| Spreadsheet workbook of the legacy binary kind (BIFF8) read | `crates/xls/Cargo.toml:3` | absent (only the OOXML workbook reader) | PORT |
| Windows metafiles (WMF, EMF, EMF+) read and write into a neutral vector picture | `crates/metafile/Cargo.toml:3` | absent | PORT |
| Machine output languages: Grbl G-code writer, HP-GL / DMPL / GP-GL plotter commands | `crates/gcode/Cargo.toml:3`, `crates/plotter/Cargo.toml:3` | absent | PORT |
| Settings that survive: app-owned TOML settings, atomic saves, damaged-file recovery | `crates/settings/Cargo.toml:3`, `crates/atomic/Cargo.toml:3` | absent | PORT |
| PostScript/EPS interpreter (paints EPS into PDF or a raster) | `crates/ps/Cargo.toml:3` | partial — craftkit exports EPS, nothing reads or interprets it | PORT |
| OpenDocument spreadsheets (.ods) read | `crates/ods/Cargo.toml:3` | absent | LATER |
| WordprocessingML document (.docx) read and write over a shared word model | `crates/docx/Cargo.toml:3`, `crates/wordmodel/Cargo.toml:3` | absent | LATER |
| Layered image format (.psd/.psb) lossless read and write | `crates/psd/Cargo.toml:3` | absent | LATER |
| Camera raw containers (CR2, NEF, ARW, RAF, DNG) to a neutral raw image | `crates/raw/Cargo.toml:3` | absent | LATER |
| Audio codecs beyond WAV (FLAC, MP3, AAC, ALAC, Opus, Vorbis, AC-3) | `crates/audio/Cargo.toml:4` | partial (only the WAV reader and writer) | LATER |
| ICC colour management: profile parse/write, transforms, intents, LUTs, gamut checks | `crates/cms/Cargo.toml:9` | absent | LATER |
| Readers for other tools' native design containers | `crates/fig/Cargo.toml:3`, `crates/sketch/Cargo.toml:3`, `crates/penpot/Cargo.toml:3`, `crates/xcf/Cargo.toml:3` | absent | LATER |
| Shared egui widget and design-system library | `crates/ui/Cargo.toml:4` | absent | LATER |
| Mail, cloud, presence/chat, e-sign and AI-provider abstractions | `crates/proto/Cargo.toml:3`, `crates/cloud/Cargo.toml:3`, `crates/review/Cargo.toml:3`, `crates/esign/Cargo.toml:3`, `crates/ai/Cargo.toml:3` | no home | SKIP |

Eight rows are PORT, and the settings row is two arcs — the settings file
itself, and the atomic replacement of a file — so the ports are nine arcs.

The LATER rows are named but not planned here. The OpenDocument workbook, the
word-processing document format and its model, the layered image format, camera
raw, the other audio codecs, colour management, the foreign design containers
and the shared widget library are each a project of its own rather than a
command, and each would deserve its own plan before it deserves an arc.

The SKIP row is out of scope for a reason that is not technical: mail, cloud,
presence, e-sign and the AI-provider abstractions are vendor services and
collaboration, and neither is this tree's business.

## 2. The arcs

| arc | rule | from | to | module | command | test | section |
|---|---|---|---|---|---|---|---|
| **A** | TIFF is read and written by one shared library | `docs/plans/craft-libs.md` | `craftkit-tiff` | `src/lib.rs` | `craftkit-tiff::Tiff::read` | `crates/craftkit-tiff/tests/tiff.rs` | §1252 |
| **B** | WebP is read and written by one shared library | `docs/plans/craft-libs.md` | `craftkit-webp` | `src/lib.rs` | `craftkit-webp::WebP::read` | `crates/craftkit-webp/tests/webp.rs` | §1253 |
| **C** | every mesh container is read into one neutral mesh model | `docs/plans/craft-libs.md` | `craftkit-mesh` | `src/lib.rs` | `craftkit-mesh::Mesh::read` | `crates/craftkit-mesh/tests/mesh.rs` | §1254 |
| **D** | the legacy binary workbook is read by the grid battery | `docs/plans/craft-libs.md` | `craftkit-plugin-grid` | `src/xls.rs` | `grid.workbook.import.xls` | `crates/craftkit-plugin-grid/tests/xls.rs` | §1255 |
| **E** | a metafile is read and written into one neutral vector picture | `docs/plans/craft-libs.md` | `craftkit-metafile` | `src/lib.rs` | `craftkit-metafile::Metafile::read` | `crates/craftkit-metafile/tests/metafile.rs` | §1256 |
| **F** | the machine-output languages are written by the vector battery | `docs/plans/craft-libs.md` | `craftkit-plugin-vector` | `src/machine_output.rs` | `vector.machine.output.export` | `crates/craftkit-plugin-vector/tests/machine_output.rs` | §1257 |
| **G** | the settings file is read and written by one shared library | `docs/plans/craft-libs.md` | `craftkit-settings` | `src/lib.rs` | `craftkit-settings::Settings::save` | `crates/craftkit-settings/tests/settings.rs` | §1258 |
| **H** | a file is replaced atomically, or not at all | `docs/plans/craft-libs.md` | `craftkit-atomic` | `src/lib.rs` | `craftkit-atomic::replace` | `crates/craftkit-atomic/tests/atomic.rs` | §1259 |
| **I** | an EPS file is interpreted, not only written | `docs/plans/craft-libs.md` | `craftkit-plugin-vector` | `src/eps.rs` | `vector.eps.import` | `crates/craftkit-plugin-vector/tests/eps.rs` | §1260 |

**Six of the nine arcs land in new shared library crates** — `craftkit-tiff`,
`craftkit-webp`, `craftkit-mesh`, `craftkit-metafile`, `craftkit-settings` and
`craftkit-atomic` — a library named for the format or the job it does, at the
layer the file formats live in. **Three land in an existing battery crate**:
the legacy workbook in the grid battery, and the machine-output languages and
the EPS reader in the vector battery.

An arc that adds a crate carries **three extra edits** on top of its own three:
the workspace `Cargo.toml`, the row in the layer table, and the row in
`docs/architecture.md`. A crate that is not in the layer table is a finding, so
an arc that added a crate and forgot those edits would not build.

The arcs reserve **§1252–§1260** of the README, one section per arc, in
order.

The arcs are in the survey's order, and they are deliberately not one
capability per arc in the abstract: the TIFF, WebP and mesh libraries are the
formats themselves, the workbook and EPS arcs are where a document already
lives, and the two settings arcs are small enough to be done properly before
anything depends on them.

## 3. The order, and what each arc ships

| arc | ships | cost |
|---|---|---|
| A | the TIFF and BigTIFF reader and writer, with the IFD chain, the Exif and GPS pointers, strips and tiles | medium — a container with many tags, and a round trip |
| B | the WebP reader and writer, with the canvas, alpha and animation frames | medium — a compressed format and an animation model |
| C | the neutral mesh model and the five container readers that fill it | large — one model and five parsers |
| D | the legacy binary workbook read, inside the grid battery | medium — a second workbook dialect, in the battery that already has one |
| E | the metafile reader and writer, producing a neutral vector picture | medium — a record format with drawing records |
| F | the machine-output writers: Grbl G-code, and the plotter languages | medium — two command vocabularies and a path to a plotter |
| G | the app-owned TOML settings file, with recovery from a damaged file | small — a small file format and a policy |
| H | the atomic replacement of a file, with the write-to-temporary and rename | small — one system call sequence, carefully |
| I | the EPS interpreter, painting into a raster or a PDF | large — a whole language, with a stack |

The order runs from the self-contained formats to the ones that reach the
document model. A, B and E are libraries with no battery in them and can land
in any order; C is the largest and wants the model settled first; D, F and I
touch batteries and so want the libraries they call to exist; G and H are small
and are worth doing early, because H is what makes G's save safe and G is what
every future front end will want.

## 4. The tests

| arc | the test that must exist |
|---|---|
| A | a TIFF round-trips: the same image written and read back is the same image; a BigTIFF file is read; a file with an unknown tag is read, not refused; a truncated file is refused, not faked |
| B | a WebP round-trips, with the alpha and the frames; a lossy image is read to the same canvas; a file with a broken chunk is refused |
| C | every container fills the same neutral mesh: the same solid in STL, in OBJ and in glTF gives the same model; a mesh with no geometry is refused; a container naming a missing external file is refused |
| D | the legacy workbook is read into the grid battery's own workbook model, cell for cell; a file that is not a workbook is refused; the battery's existing workbook tests still pass |
| E | a metafile is read into the neutral vector picture and written back; the records that carry no geometry are skipped; a malformed record is refused |
| F | the G-code writer emits the moves of a path, and the plotter writers emit their own vocabularies; a path with a move and no draw is refused |
| G | settings are written, read back, and equal; a damaged settings file is recovered by rule and the recovery is reported; a settings file with an unknown key is read without failing |
| H | the replacement is atomic: a reader never sees a half-written file; a failure before the rename leaves the old file untouched; the temporary file is taken back |
| I | an EPS is interpreted into a picture; an unknown operator is refused; a malformed file is refused rather than half-painted |

## 5. What it does not do

The rows the survey rated LATER are not in this plan, each for its own reason.
The **OpenDocument workbook** is a third workbook dialect, and one is already
in the tree. The **word-processing document format and its model** is a document
model of its own, not a format reader. The **layered image format** is lossless
editing of a layered picture, which is a different capability from reading a
flat one. **Camera raw** needs a demosaicing and a tone curve, which is a
project rather than a container. The **other audio codecs** are several
compressors and a set of channel layouts. **Colour management** is profiles,
intents and transforms, and it touches everything that has a colour. The
**foreign design containers** are readers for other tools' private formats, and
a neutral model for them does not exist here yet. The **shared widget
library** is a user-interface dependency, and this tree draws its own widgets
in code.

**Mail, cloud, presence, e-sign and the AI-provider abstractions** are out of
scope entirely: they are vendor services and collaboration features, and
neither belongs in this tree.

## 6. The open questions

The upstream has no roadmap and no gap document, so its own priorities are
only in its README and its changelog, and this plan reads priorities out of
what the crates exist.

The survey did not read the upstream's 238 crate READMEs individually. A crate
whose description hides its user-visible surface may hide a capability that is
ranked too low here, or not at all.

The survey did not verify craftkit's absence exhaustively. Its greps were
format-name based, so a format craftkit supports under an unexpected name may
be misreported as absent.

The survey did not read the whole of the cad battery, so it cannot say whether
a three-dimensional notion hides there — which is the same question Arc C
touches, and Arc C's model may have to wait for that answer.
