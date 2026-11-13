//! The **prose battery's registry**, pinned.
//!
//! # Why this test exists
//!
//! §865 converted this battery from one declaration per command to
//! **one table per file** — the `commands!` table of the derive. A
//! conversion that changed a single id, a label, a layer, an icon, a menu
//! path, a parameter or a flag would be a conversion that changed the
//! tree.
//!
//! So the tree's own registry is **pinned**: every command of this battery
//! is written down here — its id, its label, its area, its capability, its
//! layer, its icon, its documentation, its chord, its menu, its parameters
//! and its flags — and this test says whether the registry still says it.
//!
//! The snapshot was taken **before** the conversion, from the registry
//! itself. So the test is not a description of what the table says; it is
//! a description of **what the tree was**, and the table has to agree with
//! it.
//!
//! # Why it is here and not in the battery
//!
//! A battery's own test binary sees **no commands**: the plugin's items
//! are behind its anchor, and the anchor keeps them for a program that
//! links the plugin. The studio links the batteries, so the studio's
//! ecosystem is where the registry of a battery can be read.

#![cfg(feature = "prose")]

use craftkit_sdk::command::CommandSpec;

/// The commands of the prose battery, as the registry held them.
///
/// **The snapshot**: taken before the conversion, from the registry.
const EXPECTED: &[&str] = &[
    r##"prose.accessibility | Accessibility | prose | command | prose | list | **Check** a document: what makes it hard to use. | - | prose | document(node-ref)! help="The document." | mutates=false | undoable=false | journal=true | example=false"##,
    r##"prose.authority.mark | Mark | prose | command | prose | tag | **Mark** a block with an authority. | - | prose | block(node-ref)! help="The block.",entry(text)! help="The citation of the authority." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"prose.block.bookmark | Bookmark | prose | command | prose | tag | Put a **bookmark** on a block. | - | prose | block(node-ref)! help="The block.",name(text)! help="The name of the place." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"prose.block.bookmark.delete | Delete | prose | command | prose | minus | **Take away** the bookmark of a block. | - | prose | block(node-ref)! help="The block." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"prose.block.caption | Caption | prose | command | prose | tag | **Say** what a **caption** captions. | - | prose | block(node-ref)! help="The paragraph.",label(text)? help="The label: what the thing is called.",text(text)? help="What the caption says.",format(text)? help="The format of the number: `arabic`, `roman`, `alphabetic` or `ALPHABETIC`.",chapter(number)? help="The **chapter** the caption is in: the level of a heading, one to nine.",separator(text)? help="The **separator** between the chapter number and the number of the caption: `-`, `.`, `:`, an em dash or an en dash." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"prose.block.case | Case | prose | command | prose | pencil | **Change** the **case** of the text of a paragraph. | - | prose | block(node-ref)! help="The paragraph.",mode(text)! help="The way the case is changed: `upper`, `lower`, `toggle`, `title` or `sentence`." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"prose.block.character | Character | prose | command | prose | list | **Read** the character of a block: the character of the style it is on. | - | prose | block(node-ref)! help="The block." | mutates=false | undoable=false | journal=true | example=false"##,
    r##"prose.block.dashes | Dashes | prose | command | prose | pencil | **Make** the **dashes** of a paragraph typographic. | - | prose | block(node-ref)! help="The paragraph." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"prose.block.endnotes | Endnotes | prose | command | prose | list | **Read** the endnotes of a block. | - | prose | block(node-ref)! help="The block." | mutates=false | undoable=false | journal=true | example=false"##,
    r##"prose.block.equation.add | Add | prose | command | prose | plus | **Put** an equation in a document. | - | prose | document(node-ref)! help="The document it goes in.",parts(json)! help="The **parts** of the equation: a text, a fraction, a superscript, a subscript or a radical." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"prose.block.field.add | Add | prose | command | prose | link | Put a **field** on a block. | - | prose | block(node-ref)! help="The block.",start(number)! help="Where the field begins, in characters.",end(number)! help="Where the field ends, in characters.",code(text)! help="The instruction of the field.",locked(bool)? help="Whether the field is locked against change." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"prose.block.field.read | Read | prose | command | prose | list | **Read** the **fields** of a block, each with the **result** it covers. | - | prose | block(node-ref)! help="The block." | mutates=false | undoable=false | journal=true | example=false"##,
    r##"prose.block.field.set | Set | prose | command | prose | pencil | **Change** a **field** of a block. | - | prose | block(node-ref)! help="The block.",field(number)! help="The field: the character it begins at.",code(text)? help="The instruction of the field.",locked(bool)? help="Whether the field is locked against change." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"prose.block.field.unlink | Unlink | prose | command | prose | trash | **Unlink** a **field** of a block. | - | prose | block(node-ref)! help="The block.",field(number)! help="The field: the character it begins at." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"prose.block.footnotes | Footnotes | prose | command | prose | list | **Read** the footnotes of a block. | - | prose | block(node-ref)! help="The block." | mutates=false | undoable=false | journal=true | example=false"##,
    r##"prose.block.links | Links | prose | command | prose | link | The **links** of a block. | - | prose | block(node-ref)! help="The block." | mutates=false | undoable=false | journal=true | example=false"##,
    r##"prose.block.list | List | prose | command | prose | list | **Make** the **list** a paragraph begins. | - | prose | block(node-ref)! help="The paragraph." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"prose.block.move | Move | prose | command | prose | arrow | **Move** a block to another place in its document. | - | prose | block(node-ref)! help="The block.",to(number)! help="Which place it goes to, counting from one among the blocks of the document." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"prose.block.number | Number | prose | command | prose | list | **Read** the number of a block. | - | prose | block(node-ref)! help="The block." | mutates=false | undoable=false | journal=true | example=false"##,
    r##"prose.block.numbering | Numbering | prose | command | prose | list | Put a **block** in a list, at a level. | - | prose | block(node-ref)! help="The block.",list(node-ref)! help="The list.",level(number)? help="The level, counting from zero. Without it the block is at the first level." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"prose.block.paragraph | Paragraph | prose | command | prose | list | **Read** the paragraph of a block: the paragraph of the style it is on. | - | prose | block(node-ref)! help="The block." | mutates=false | undoable=false | journal=true | example=false"##,
    r##"prose.block.paragraph.add | Add | prose | command | prose | plus | Put a **paragraph** in a document. | - | prose | document(node-ref)! help="The document it goes in.",text(text)! help="What it says." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"prose.block.paragraph.align | Align | prose | command | prose | align | Say how a paragraph **stands**. | - | prose | block(node-ref)! help="The paragraph.",align(text)! help="How it stands: `left`, `center`, `right`, `justify` or `distribute`." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"prose.block.paragraph.indent | Indent | prose | command | prose | indent | Say how far a paragraph is **indented**, in points. | - | prose | block(node-ref)! help="The paragraph.",left(number)? help="How far from the left edge, in points.",right(number)? help="How far from the right edge, in points.",first_line(number)? help="How far the first line is in from the left, in points. A negative number is a **hanging** indent." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"prose.block.paragraph.linespacing | Linespacing | prose | command | prose | lines | Say how the **lines** of a paragraph are spaced. | - | prose | block(node-ref)! help="The paragraph.",multiple(number)? help="A multiple of single spacing: `1` is single, `1.5` is one and a half, `2` is double.",at_least(number)? help="At least this many points.",exactly(number)? help="Exactly this many points." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"prose.block.paragraph.spacing | Spacing | prose | command | prose | space | Say how much **space** is around a paragraph, in points. | - | prose | block(node-ref)! help="The paragraph.",before(number)? help="How much space is above it, in points.",after(number)? help="How much space is below it, in points." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"prose.block.paragraph.text | Text | prose | command | prose | pencil | Change **what a paragraph says**. | - | prose | block(node-ref)! help="The paragraph.",text(text)! help="What it says now." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"prose.block.quotes | Quotes | prose | command | prose | pencil | **Make** the **quotes** of a paragraph smart. | - | prose | block(node-ref)! help="The paragraph." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"prose.block.reference.add | Add | prose | command | prose | plus | **Add** a cross-reference to a document. | - | prose | document(node-ref)! help="The document it goes in.",target(text)! help="The **bookmark** it points at — or the **number** or the **text** of the caption it points at.",to(text)? help="What it points at.",show(text)? help="How it shows what it points at." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"prose.block.reference.read | Read | prose | command | prose | list | **Read** the text of a block: the text it shows, which for a reference is the text of the block it points at. | - | prose | block(node-ref)! help="The block." | mutates=false | undoable=false | journal=true | example=false"##,
    r##"prose.block.reference.set | Set | prose | command | prose | pencil | **Point** a cross-reference at another bookmark. | - | prose | block(node-ref)! help="The reference.",target(text)! help="The **bookmark** it points at — or the **number** or the **text** of the caption it points at.",to(text)? help="What it points at.",show(text)? help="How it shows what it points at." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"prose.block.remove | Remove | prose | command | prose | trash | Take a **block** out of a document. | - | prose | block(node-ref)! help="The block." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"prose.block.revisions | Revisions | prose | command | prose | list | The **revisions** of a block. | - | prose | block(node-ref)! help="The block." | mutates=false | undoable=false | journal=true | example=false"##,
    r##"prose.block.runs | Runs | prose | command | prose | list | Set the **runs** of a block: the formatting of its text. | - | prose | block(node-ref)! help="The block.",runs(json)! help="The runs: a list of `{len, bold, italic, underline, colour, size}`." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"prose.block.stops | Stops | prose | command | prose | pencil | **Make** the **full stop** of a paragraph. | - | prose | block(node-ref)! help="The paragraph." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"prose.block.style | Style | prose | command | prose | plus | Put a **style** on a block. | - | prose | block(node-ref)! help="The block.",style(node-ref)! help="The style." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"prose.block.table.add | Add | prose | command | prose | plus | Put a **table** in a document. | - | prose | document(node-ref)! help="The document it goes in.",rows(json)? help="The rows, each a list of cells." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"prose.citation.styles | Styles | prose | command | prose | list | **Read** the **citation styles** of this tree. | - | prose |  | mutates=false | undoable=false | journal=true | example=false"##,
    r##"prose.comment.delete | Delete | prose | command | prose | trash | **Take away** the comment on a block. | - | prose | block(node-ref)! help="The block." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"prose.comment.get | Get | prose | command | prose | list | **Read** the comment on a block. | - | prose | block(node-ref)! help="The block." | mutates=false | undoable=false | journal=true | example=false"##,
    r##"prose.comment.set | Set | prose | command | prose | quote | **Put** a comment on a block. | - | prose | block(node-ref)! help="The block the comment is on.",author(text)! help="Who said it.",text(text)! help="What they said." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"prose.document.add | Add | prose | command | prose | plus | Put a **document** in the document. | - | prose | name(text)! help="What to call it." | mutates=true | undoable=true | journal=true | example=true"##,
    r##"prose.document.authorities | Authorities | prose | command | prose | list | The **table of authorities** of a document: the entries it has been marked with, and the blocks they are on. | - | prose | document(node-ref)! help="The document." | mutates=false | undoable=false | journal=true | example=false"##,
    r##"prose.document.bibliography | Bibliography | prose | command | prose | list | **Read** the **bibliography** of a document: the sources that are cited, written in the style of the document. | - | prose | document(node-ref)! help="The document." | mutates=false | undoable=false | journal=true | example=false"##,
    r##"prose.document.bookmarks | Bookmarks | prose | command | prose | list | **Read** the bookmarks of a document. | - | prose | document(node-ref)! help="The document." | mutates=false | undoable=false | journal=true | example=false"##,
    r##"prose.document.captions | Captions | prose | command | prose | list | **Read** the **captions** of a document, each with the **number** it has. | - | prose | document(node-ref)! help="The document." | mutates=false | undoable=false | journal=true | example=false"##,
    r##"prose.document.citation | Citation | prose | command | prose | list | **Read** the **citation** of a source, in the style of its document. | - | prose | document(node-ref)! help="The document the source belongs to.",tag(text)! help="The tag that names the source.",pages(text)? help="The pages of the source that are cited." | mutates=false | undoable=false | journal=true | example=false"##,
    r##"prose.document.citestyle.read | Read | prose | command | prose | list | **Read** the **citation style** of a document. | - | prose | document(node-ref)! help="The document." | mutates=false | undoable=false | journal=true | example=false"##,
    r##"prose.document.citestyle.set | Set | prose | command | prose | pencil | **Say** which **citation style** a document cites by. | - | prose | document(node-ref)! help="The document.",style(text)! help="The name of the style." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"prose.document.contents | Contents | prose | command | prose | list | **Read** the **table of contents** of a document: its headings, with the level each of them is at. | - | prose | document(node-ref)! help="The document.",levels(number)? help="The levels that are in the table: three of them, one to three, unless the reader asks for others." | mutates=false | undoable=false | journal=true | example=false"##,
    r##"prose.document.figures | Figures | prose | command | prose | list | **Read** the **table of figures** of a document: the captions that carry one label. | - | prose | document(node-ref)! help="The document.",label(text)? help="The label of the captions: `Figure` when it is not given.",include_label(bool)? help="Whether the entries carry the label: yes, unless it is said no." | mutates=false | undoable=false | journal=true | example=false"##,
    r##"prose.document.footer | Footer | prose | command | prose | pencil | **Say** what a document carries at the bottom of every page. | - | prose | document(node-ref)! help="The document.",footer(text)! help="The footer's text." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"prose.document.footer.read | Read | prose | command | prose | list | **Read** the **footer** of a document. | - | prose | document(node-ref)! help="The document." | mutates=false | undoable=false | journal=true | example=false"##,
    r##"prose.document.header | Header | prose | command | prose | pencil | **Say** what a document carries at the top of every page. | - | prose | document(node-ref)! help="The document.",header(text)! help="The header's text." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"prose.document.header.read | Read | prose | command | prose | list | **Read** the **header** of a document. | - | prose | document(node-ref)! help="The document." | mutates=false | undoable=false | journal=true | example=false"##,
    r##"prose.document.outline | Outline | prose | command | prose | list | **Read** the outline of a document: its headings, in the order they stand. | - | prose | document(node-ref)! help="The document." | mutates=false | undoable=false | journal=true | example=false"##,
    r##"prose.document.properties | Properties | prose | command | prose | tag | **Say** what a document is. | - | prose | document(node-ref)! help="The document.",author(text)? help="Who wrote it.",subject(text)? help="What it is about.",keywords(text)? help="The words a search looks for.",comments(text)? help="What a reader may say about it.",category(text)? help="The category it is in.",last_modified_by(text)? help="Who touched it last.",revision(number)? help="How many times it has been revised.",custom(json)? help="The **custom** properties: a list of pairs, each a name and a value." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"prose.document.source.read | Read | prose | command | prose | list | **Read** the **sources** of a document. | - | prose | document(node-ref)! help="The document." | mutates=false | undoable=false | journal=true | example=false"##,
    r##"prose.document.source.remove | Remove | prose | command | prose | trash | **Take away** a **source** of a document. | - | prose | document(node-ref)! help="The document.",tag(text)! help="The tag that names the source." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"prose.document.source.set | Set | prose | command | prose | tag | **Say** what a **source** is. | - | prose | document(node-ref)! help="The document.",tag(text)? help="The tag that names the source.",kind(text)? help="The kind of the source.",author(text)? help="Who wrote it.",title(text)? help="What it is called.",year(text)? help="When it was made.",publisher(text)? help="Who published it.",city(text)? help="Where it was made.",journal(text)? help="The journal it is in.",volume(text)? help="The volume of the journal it is in.",pages(text)? help="The pages of it.",url(text)? help="Where it may be found." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"prose.document.stats | Stats | prose | command | prose | list | **Read** the **statistics** of a document. | - | prose | document(node-ref)! help="The document to measure." | mutates=false | undoable=false | journal=true | example=false"##,
    r##"prose.document.theme | Theme | prose | command | prose | paint | **Say** what the **theme** of a document is. | - | prose | document(node-ref)! help="The document.",major(text)? help="The font of the headings.",minor(text)? help="The font of the body.",accent(text)? help="The accent colour of the headings, as `#rrggbb`." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"prose.document.title | Title | prose | command | prose | pencil | **Say** what a document is. | - | prose | document(node-ref)! help="The document.",title(text)! help="The title." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"prose.endnote.add | Add | prose | command | prose | plus | Put an **endnote** in a document. | - | prose | document(node-ref)! help="The document it goes in.",text(text)! help="What it says." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"prose.equation.set | Set | prose | command | prose | pencil | **Set** the parts of an equation. | - | prose | block(node-ref)! help="The equation.",parts(json)! help="The **parts** of the equation." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"prose.find | Find | prose | command | prose | list | **Find** a pattern in a block. | - | prose | block(node-ref)! help="The block.",what(text)! help="The pattern to find.",wildcards(bool)? help="Whether the pattern holds **wildcards**: a `*` for any text and a `?` for one character.",case(bool)? help="Whether the **case** matters." | mutates=false | undoable=false | journal=true | example=false"##,
    r##"prose.footnote.add | Add | prose | command | prose | plus | Put a **footnote** in a document. | - | prose | document(node-ref)! help="The document it goes in.",text(text)! help="What it says." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"prose.index.mark | Mark | prose | command | prose | tag | **Mark** a block with an index **entry**. | - | prose | block(node-ref)! help="The block.",entry(text)! help="The entry.",subentry(text)? help="The subentry under the entry." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"prose.index.read | Read | prose | command | prose | list | **Read** the **index** of a document. | - | prose | document(node-ref)! help="The document." | mutates=false | undoable=false | journal=true | example=false"##,
    r##"prose.list.add | Add | prose | command | prose | plus | Put a **list** in a document. | - | prose | document(node-ref)! help="The document it goes in.",kind(text)! help="The kind of list: `bullet`, `numbered`, `numberedParen`, `outline`, `upperLetter`, `lowerLetter`, `lowerLetterDot`, `lowerRoman` or `legal` — or a single character, which is a bullet of its own." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"prose.media.add | Add | prose | command | prose | plus | Put a **media** in a document: a file the document uses. | - | prose | document(node-ref)! help="The document it goes in.",name(text)! help="What to call it.",path(text)! help="Where the file is, relative to the document." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"prose.media.alt | Alt | prose | command | prose | pencil | **Say** what a picture is. | - | prose | media(node-ref)! help="The picture.",alt(text)! help="The text that stands for it." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"prose.media.decorative | Decorative | prose | command | prose | pencil | **Say** that a picture is **decorative**. | - | prose | media(node-ref)! help="The picture.",decorative(bool)? help="Whether the picture is decorative." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"prose.media.placement | Placement | prose | command | prose | pencil | **Say** where a picture **sits**. | - | prose | media(node-ref)! help="The picture.",placement(text)? help="Where the picture sits: `inline` or `floating`." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"prose.merge | Merge | prose | command | prose | list | **Merge** the fields of a document with the rows of a data source. | - | prose | headers(json)? help="The names of the columns of the data source.",rows(json)! help="The rows of the data source: a list of lists of values." | mutates=false | undoable=false | journal=true | example=false"##,
    r##"prose.notes.convert | Convert | prose | command | prose | pencil | **Convert** the **notes** of a document. | - | prose | document(node-ref)! help="The document whose notes are converted.",to(text)! help="What the notes become: `endnote`, `footnote` or `swap`." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"prose.number.format | Format | prose | command | prose | list | **Read** the **numbering format** of a number. | - | prose | value(number)! help="The number to write.",format(text)! help="The form: `roman`, `letters`, `ordinal` or `cardinal`." | mutates=false | undoable=false | journal=true | example=false"##,
    r##"prose.page.background | Background | prose | command | prose | paint | **Paint** the page of a document with a **colour**. | - | prose | document(node-ref)! help="The document.",colour(text)! help="The colour of the page, as `#rrggbb`." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"prose.page.borders | Borders | prose | command | prose | rect | **Set** the **borders** of the page of a document. | - | prose | document(node-ref)! help="The document.",kind(text)? help="`box` for a border on every side, `none` for none at all.",style(text)? help="The style of the border.",width(number)? help="The width of the border, in points.",colour(text)? help="The colour of the border, as `#rrggbb`.",sides(json)? help="The borders of the sides named: `{\"top\": {…}, \"left\": null}`." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"prose.page.columns | Columns | prose | command | prose | rect | **Set** the **columns** of the page of a document. | - | prose | document(node-ref)! help="The document.",count(number)! help="How many columns the page has, from one to twelve.",space(number)? help="The space between the columns, in points.",separator(bool)? help="Whether a line is drawn between the columns." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"prose.page.get | Get | prose | command | prose | rect | **Read** the page of a document. | - | prose | document(node-ref)! help="The document." | mutates=false | undoable=false | journal=true | example=false"##,
    r##"prose.page.numbers | Numbers | prose | command | prose | list | **Set** the **numbering** of the pages. | - | prose | document(node-ref)! help="The document.",format(text)? help="The format the numbers are written in.",start(number)? help="The number the first page has." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"prose.page.set | Set | prose | command | prose | rect | **Set** the page of a document. | - | prose | document(node-ref)! help="The document.",width(number)! help="The width of the page, in points.",height(number)! help="The height of the page, in points.",top(number)? help="The margin at the top, in points.",bottom(number)? help="The margin at the bottom, in points.",left(number)? help="The margin on the left, in points.",right(number)? help="The margin on the right, in points." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"prose.page.watermark | Watermark | prose | command | prose | paint | **Put** a **watermark** on the page of a document. | - | prose | document(node-ref)! help="The document.",text(text)! help="What the watermark says.",diagonal(bool)? help="Whether the watermark runs **across** the page.",colour(text)? help="The colour of the watermark, as `#rrggbb`." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"prose.replace | Replace | prose | command | prose | pencil | **Replace** a pattern in a block. | - | prose | block(node-ref)! help="The block.",what(text)! help="The pattern to find.",with(text)! help="What to put in its place.",wildcards(bool)? help="Whether the pattern holds **wildcards**: a `*` for any text and a `?` for one character.",case(bool)? help="Whether the **case** matters." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"prose.revision.accept | Accept | prose | command | prose | check | **Accept** a revision. | - | prose | block(node-ref)! help="The block.",run(number)! help="The run the revision is on: its number, counting from nothing." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"prose.revision.accept.all | All | prose | command | prose | check | **Accept** every revision of a block. | - | prose | block(node-ref)! help="The block." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"prose.revision.reject | Reject | prose | command | prose | undo | **Reject** a revision. | - | prose | block(node-ref)! help="The block.",run(number)! help="The run the revision is on: its number, counting from nothing." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"prose.revision.reject.all | All | prose | command | prose | undo | **Reject** every revision of a block. | - | prose | block(node-ref)! help="The block." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"prose.run.format | Format | prose | command | prose | pencil | The **character** of a piece of text. | - | prose | block(node-ref)! help="The block the text is in.",start(number)! help="Where the text begins, counted in characters.",length(number)! help="How long the text is, counted in characters.",bold(bool)? help="Whether the text is **bold**.",italic(bool)? help="Whether the text is **italic**.",underline(bool)? help="Whether the text is **underlined**.",colour(text)? help="The **colour** of the text, as `#rrggbb`.",size(number)? help="The **size** of the text." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"prose.style.add | Add | prose | command | prose | plus | Put a **style** in a document. | - | prose | document(node-ref)! help="The document it goes in.",name(text)! help="The style's name.",style_kind(text)? help="What the style applies to: `paragraph`, `character`, `table` or `numbering` — or anything else, because the list is open.",based_on(node-ref)? help="The style it is based on." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"prose.style.character | Character | prose | command | prose | pencil | **Set** the character of a style. | - | prose | style(node-ref)! help="The style.",font(text)? help="The **font** of the text.",size(number)? help="The **size** of the text.",colour(text)? help="The **colour** of the text, as `#rrggbb`.",bold(bool)? help="Whether the text is **bold**.",italic(bool)? help="Whether the text is **italic**.",underline(bool)? help="Whether the text is **underlined**." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"prose.style.character.read | Read | prose | command | prose | list | **Read** the character of a style, with the character of its base under it. | - | prose | style(node-ref)! help="The style." | mutates=false | undoable=false | journal=true | example=false"##,
    r##"prose.style.delete | Delete | prose | command | prose | minus | **Delete** a **style**. | - | prose | style(node-ref)! help="The style to delete." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"prose.style.level | Level | prose | command | prose | list | **Set** the level of a style — or take it away. | - | prose | style(node-ref)! help="The style.",level(number)? help="The level: a whole number from 1 to 9. Without it, the style has no level of its own." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"prose.style.level.read | Read | prose | command | prose | list | **Read** the level of a style, with the level of its base under it. | - | prose | style(node-ref)! help="The style." | mutates=false | undoable=false | journal=true | example=false"##,
    r##"prose.style.list | List | prose | command | prose | list | **Read** the **styles** of a document. | - | prose | document(node-ref)! help="The document to look in." | mutates=false | undoable=false | journal=true | example=false"##,
    r##"prose.style.paragraph | Paragraph | prose | command | prose | pencil | **Set** the paragraph of a style. | - | prose | style(node-ref)! help="The style.",alignment(text)? help="The **alignment** of the paragraph.",space_before(number)? help="The room **above** the paragraph, in points.",space_after(number)? help="The room **below** the paragraph, in points.",line_spacing(number)? help="The **line spacing**, as a multiple of the line.",indent_left(number)? help="The **indent** of the paragraph, in points.",indent_right(number)? help="The **indent** at the right of the paragraph, in points.",indent_first(number)? help="The **indent of the first line**, in points. A negative one hangs the line." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"prose.style.paragraph.read | Read | prose | command | prose | list | **Read** the paragraph of a style, with the paragraph of its base under it. | - | prose | style(node-ref)! help="The style." | mutates=false | undoable=false | journal=true | example=false"##,
    r##"prose.table.alt | Alt | prose | command | prose | pencil | **Say** what a table is. | - | prose | table(node-ref)! help="The table.",alt(text)! help="The text that stands for it." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"prose.table.cell.get | Get | prose | command | prose | list | **Read** a cell of a table. | - | prose | table(node-ref)! help="The table.",row(number)! help="The row the cell is in, counting from nothing.",column(number)! help="The column the cell is in, counting from nothing." | mutates=false | undoable=false | journal=true | example=false"##,
    r##"prose.table.cell.set | Set | prose | command | prose | pencil | **Set** a cell of a table. | - | prose | table(node-ref)! help="The table.",row(number)! help="The row the cell is in, counting from nothing.",column(number)! help="The column the cell is in, counting from nothing.",value(json)! help="What the cell holds." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"prose.table.column.add | Add | prose | command | prose | plus | Put a **column** in a table. | - | prose | table(node-ref)! help="The table.",at(number)? help="Where the column goes: the columns beside it move right. The end of the table when it is not said." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"prose.table.column.remove | Remove | prose | command | prose | minus | Take a **column** out of a table. | - | prose | table(node-ref)! help="The table.",column(number)! help="The column that goes." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"prose.table.header | Header | prose | command | prose | list | **Say** whether the first row of a table is a header row. | - | prose | table(node-ref)! help="The table.",header(bool)! help="Whether the first row is a header row." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"prose.table.merge | Merge | prose | command | prose | link | **Merge** cells of a table into one. | - | prose | table(node-ref)! help="The table.",row(number)! help="The row the merge begins at, counting from nothing.",column(number)! help="The column the merge begins at, counting from nothing.",rows(number)? help="How many rows it reaches down. It is one unless the file says another.",columns(number)? help="How many columns it reaches across. It is one unless the file says another." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"prose.table.merges | Merges | prose | command | prose | list | **Read** the **merged cells** of a table. | - | prose | table(node-ref)! help="The table." | mutates=false | undoable=false | journal=true | example=false"##,
    r##"prose.table.row.add | Add | prose | command | prose | plus | Put a **row** in a table. | - | prose | table(node-ref)! help="The table.",at(number)? help="Where the row goes: the rows below it move down. The end of the table when it is not said." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"prose.table.row.remove | Remove | prose | command | prose | minus | Take a **row** out of a table. | - | prose | table(node-ref)! help="The table.",row(number)! help="The row that goes." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"prose.table.split | Split | prose | command | prose | minus | **Split** a merged cell of a table. | - | prose | table(node-ref)! help="The table.",row(number)! help="The row the cell is in, counting from nothing.",column(number)! help="The column the cell is in, counting from nothing." | mutates=true | undoable=true | journal=true | example=false"##,
];

/// The commands of the prose battery, in the order of their ids.
fn the_battery() -> Vec<CommandSpec> {
    let ecosystem = craftkit_studio::ecosystem().expect("the ecosystem is not a mess");
    let mut commands: Vec<CommandSpec> =
        ecosystem.commands.all().iter().filter(|command| command.area == "prose").cloned().collect();
    commands.sort_by(|left, right| left.id.cmp(right.id));
    commands
}

/// One command, written down.
///
/// **Everything the registry carries that is data**: the id, the label, the
/// area, the capability, the layer, the icon, the documentation, the
/// chord, the menu path, the parameters — with each parameter's **help**,
/// its other spelling, its kind's own name and its unit — and the flags.
///
/// The `enabled` predicate is a **function**, and a function is not
/// something a snapshot can write down; the snapshot says everything else.
fn written(command: &CommandSpec) -> String {
    let params: Vec<String> = command
        .params
        .iter()
        .map(|param| {
            let mut text = format!(
                "{}({}){}",
                param.name,
                param.kind.json_name(),
                if param.required { "!" } else { "?" }
            );
            if !param.help.is_empty() {
                text.push_str(&format!(" help=\"{}\"", quoted(param.help)));
            }
            if let Some(alternative) = param.alternative {
                text.push_str(&format!(" or=\"{}\"", quoted(alternative)));
            }
            if let Some(type_name) = param.type_name {
                text.push_str(&format!(" type=\"{}\"", quoted(type_name)));
            }
            if let Some(unit) = param.unit {
                text.push_str(&format!(" unit=\"{}\"", quoted(unit)));
            }
            text
        })
        .collect();
    format!(
        "{} | {} | {} | {} | {} | {} | {} | {} | {} | {} | mutates={} | undoable={} | journal={} | example={}",
        command.id,
        command.label,
        command.area,
        command.capability,
        command.layer,
        command.icon,
        command.doc,
        command.shortcut.unwrap_or("-"),
        command.menu.join("/"),
        params.join(","),
        command.mutates,
        command.undoable,
        command.journal,
        command.example
    )
}

/// A text, with its backslashes and its quotes escaped.
fn quoted(text: &str) -> String {
    text.replace('\\', "\\\\").replace('"', "\\\"")
}

/// The registry says what the snapshot says.
#[test]
fn the_registry_is_the_snapshot() {
    let commands = the_battery();
    assert!(!commands.is_empty(), "the battery has no commands");
    let found: Vec<String> = commands.iter().map(written).collect();
    if std::env::var("CRAFTKIT_DUMP").is_ok() {
        for line in &found {
            println!("    r##\"{line}\"##,");
        }
        return;
    }
    assert_eq!(found, EXPECTED, "the registry is not what it was");
}
