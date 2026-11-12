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
    r#"prose.accessibility | Accessibility | prose | command | prose | list | **Check** a document: what makes it hard to use. | - | prose | document(node-ref)! | mutates=false | undoable=false"#,
    r#"prose.authority.mark | Mark | prose | command | prose | tag | **Mark** a block with an authority. | - | prose | block(node-ref)!,entry(text)! | mutates=true | undoable=true"#,
    r#"prose.block.bookmark | Bookmark | prose | command | prose | tag | Put a **bookmark** on a block. | - | prose | block(node-ref)!,name(text)! | mutates=true | undoable=true"#,
    r#"prose.block.bookmark.delete | Delete | prose | command | prose | minus | **Take away** the bookmark of a block. | - | prose | block(node-ref)! | mutates=true | undoable=true"#,
    r#"prose.block.caption | Caption | prose | command | prose | tag | **Say** what a **caption** captions. | - | prose | block(node-ref)!,label(text)?,text(text)?,format(text)?,chapter(number)?,separator(text)? | mutates=true | undoable=true"#,
    r#"prose.block.case | Case | prose | command | prose | pencil | **Change** the **case** of the text of a paragraph. | - | prose | block(node-ref)!,mode(text)! | mutates=true | undoable=true"#,
    r#"prose.block.character | Character | prose | command | prose | list | **Read** the character of a block: the character of the style it is on. | - | prose | block(node-ref)! | mutates=false | undoable=false"#,
    r#"prose.block.dashes | Dashes | prose | command | prose | pencil | **Make** the **dashes** of a paragraph typographic. | - | prose | block(node-ref)! | mutates=true | undoable=true"#,
    r#"prose.block.endnotes | Endnotes | prose | command | prose | list | **Read** the endnotes of a block. | - | prose | block(node-ref)! | mutates=false | undoable=false"#,
    r#"prose.block.equation.add | Add | prose | command | prose | plus | **Put** an equation in a document. | - | prose | document(node-ref)!,parts(json)! | mutates=true | undoable=true"#,
    r#"prose.block.field.add | Add | prose | command | prose | link | Put a **field** on a block. | - | prose | block(node-ref)!,start(number)!,end(number)!,code(text)!,locked(bool)? | mutates=true | undoable=true"#,
    r#"prose.block.field.read | Read | prose | command | prose | list | **Read** the **fields** of a block, each with the **result** it covers. | - | prose | block(node-ref)! | mutates=false | undoable=false"#,
    r#"prose.block.field.set | Set | prose | command | prose | pencil | **Change** a **field** of a block. | - | prose | block(node-ref)!,field(number)!,code(text)?,locked(bool)? | mutates=true | undoable=true"#,
    r#"prose.block.field.unlink | Unlink | prose | command | prose | trash | **Unlink** a **field** of a block. | - | prose | block(node-ref)!,field(number)! | mutates=true | undoable=true"#,
    r#"prose.block.footnotes | Footnotes | prose | command | prose | list | **Read** the footnotes of a block. | - | prose | block(node-ref)! | mutates=false | undoable=false"#,
    r#"prose.block.links | Links | prose | command | prose | link | The **links** of a block. | - | prose | block(node-ref)! | mutates=false | undoable=false"#,
    r#"prose.block.list | List | prose | command | prose | list | **Make** the **list** a paragraph begins. | - | prose | block(node-ref)! | mutates=true | undoable=true"#,
    r#"prose.block.move | Move | prose | command | prose | arrow | **Move** a block to another place in its document. | - | prose | block(node-ref)!,to(number)! | mutates=true | undoable=true"#,
    r#"prose.block.number | Number | prose | command | prose | list | **Read** the number of a block. | - | prose | block(node-ref)! | mutates=false | undoable=false"#,
    r#"prose.block.numbering | Numbering | prose | command | prose | list | Put a **block** in a list, at a level. | - | prose | block(node-ref)!,list(node-ref)!,level(number)? | mutates=true | undoable=true"#,
    r#"prose.block.paragraph | Paragraph | prose | command | prose | list | **Read** the paragraph of a block: the paragraph of the style it is on. | - | prose | block(node-ref)! | mutates=false | undoable=false"#,
    r#"prose.block.paragraph.add | Add | prose | command | prose | plus | Put a **paragraph** in a document. | - | prose | document(node-ref)!,text(text)! | mutates=true | undoable=true"#,
    r#"prose.block.paragraph.align | Align | prose | command | prose | align | Say how a paragraph **stands**. | - | prose | block(node-ref)!,align(text)! | mutates=true | undoable=true"#,
    r#"prose.block.paragraph.indent | Indent | prose | command | prose | indent | Say how far a paragraph is **indented**, in points. | - | prose | block(node-ref)!,left(number)?,right(number)?,first_line(number)? | mutates=true | undoable=true"#,
    r#"prose.block.paragraph.linespacing | Linespacing | prose | command | prose | lines | Say how the **lines** of a paragraph are spaced. | - | prose | block(node-ref)!,multiple(number)?,at_least(number)?,exactly(number)? | mutates=true | undoable=true"#,
    r#"prose.block.paragraph.spacing | Spacing | prose | command | prose | space | Say how much **space** is around a paragraph, in points. | - | prose | block(node-ref)!,before(number)?,after(number)? | mutates=true | undoable=true"#,
    r#"prose.block.paragraph.text | Text | prose | command | prose | pencil | Change **what a paragraph says**. | - | prose | block(node-ref)!,text(text)! | mutates=true | undoable=true"#,
    r#"prose.block.quotes | Quotes | prose | command | prose | pencil | **Make** the **quotes** of a paragraph smart. | - | prose | block(node-ref)! | mutates=true | undoable=true"#,
    r#"prose.block.reference.add | Add | prose | command | prose | plus | **Add** a cross-reference to a document. | - | prose | document(node-ref)!,target(text)!,to(text)?,show(text)? | mutates=true | undoable=true"#,
    r#"prose.block.reference.read | Read | prose | command | prose | list | **Read** the text of a block: the text it shows, which for a reference is the text of the block it points at. | - | prose | block(node-ref)! | mutates=false | undoable=false"#,
    r#"prose.block.reference.set | Set | prose | command | prose | pencil | **Point** a cross-reference at another bookmark. | - | prose | block(node-ref)!,target(text)!,to(text)?,show(text)? | mutates=true | undoable=true"#,
    r#"prose.block.remove | Remove | prose | command | prose | trash | Take a **block** out of a document. | - | prose | block(node-ref)! | mutates=true | undoable=true"#,
    r#"prose.block.revisions | Revisions | prose | command | prose | list | The **revisions** of a block. | - | prose | block(node-ref)! | mutates=false | undoable=false"#,
    r#"prose.block.runs | Runs | prose | command | prose | list | Set the **runs** of a block: the formatting of its text. | - | prose | block(node-ref)!,runs(json)! | mutates=true | undoable=true"#,
    r#"prose.block.stops | Stops | prose | command | prose | pencil | **Make** the **full stop** of a paragraph. | - | prose | block(node-ref)! | mutates=true | undoable=true"#,
    r#"prose.block.style | Style | prose | command | prose | plus | Put a **style** on a block. | - | prose | block(node-ref)!,style(node-ref)! | mutates=true | undoable=true"#,
    r#"prose.block.table.add | Add | prose | command | prose | plus | Put a **table** in a document. | - | prose | document(node-ref)!,rows(json)? | mutates=true | undoable=true"#,
    r#"prose.citation.styles | Styles | prose | command | prose | list | **Read** the **citation styles** of this tree. | - | prose |  | mutates=false | undoable=false"#,
    r#"prose.comment.delete | Delete | prose | command | prose | trash | **Take away** the comment on a block. | - | prose | block(node-ref)! | mutates=true | undoable=true"#,
    r#"prose.comment.get | Get | prose | command | prose | list | **Read** the comment on a block. | - | prose | block(node-ref)! | mutates=false | undoable=false"#,
    r#"prose.comment.set | Set | prose | command | prose | quote | **Put** a comment on a block. | - | prose | block(node-ref)!,author(text)!,text(text)! | mutates=true | undoable=true"#,
    r#"prose.document.add | Add | prose | command | prose | plus | Put a **document** in the document. | - | prose | name(text)! | mutates=true | undoable=true"#,
    r#"prose.document.authorities | Authorities | prose | command | prose | list | The **table of authorities** of a document: the entries it has been marked with, and the blocks they are on. | - | prose | document(node-ref)! | mutates=false | undoable=false"#,
    r#"prose.document.bibliography | Bibliography | prose | command | prose | list | **Read** the **bibliography** of a document: the sources that are cited, written in the style of the document. | - | prose | document(node-ref)! | mutates=false | undoable=false"#,
    r#"prose.document.bookmarks | Bookmarks | prose | command | prose | list | **Read** the bookmarks of a document. | - | prose | document(node-ref)! | mutates=false | undoable=false"#,
    r#"prose.document.captions | Captions | prose | command | prose | list | **Read** the **captions** of a document, each with the **number** it has. | - | prose | document(node-ref)! | mutates=false | undoable=false"#,
    r#"prose.document.citation | Citation | prose | command | prose | list | **Read** the **citation** of a source, in the style of its document. | - | prose | document(node-ref)!,tag(text)!,pages(text)? | mutates=false | undoable=false"#,
    r#"prose.document.citestyle.read | Read | prose | command | prose | list | **Read** the **citation style** of a document. | - | prose | document(node-ref)! | mutates=false | undoable=false"#,
    r#"prose.document.citestyle.set | Set | prose | command | prose | pencil | **Say** which **citation style** a document cites by. | - | prose | document(node-ref)!,style(text)! | mutates=true | undoable=true"#,
    r#"prose.document.contents | Contents | prose | command | prose | list | **Read** the **table of contents** of a document: its headings, with the level each of them is at. | - | prose | document(node-ref)!,levels(number)? | mutates=false | undoable=false"#,
    r#"prose.document.figures | Figures | prose | command | prose | list | **Read** the **table of figures** of a document: the captions that carry one label. | - | prose | document(node-ref)!,label(text)?,include_label(bool)? | mutates=false | undoable=false"#,
    r#"prose.document.footer | Footer | prose | command | prose | pencil | **Say** what a document carries at the bottom of every page. | - | prose | document(node-ref)!,footer(text)! | mutates=true | undoable=true"#,
    r#"prose.document.footer.read | Read | prose | command | prose | list | **Read** the **footer** of a document. | - | prose | document(node-ref)! | mutates=false | undoable=false"#,
    r#"prose.document.header | Header | prose | command | prose | pencil | **Say** what a document carries at the top of every page. | - | prose | document(node-ref)!,header(text)! | mutates=true | undoable=true"#,
    r#"prose.document.header.read | Read | prose | command | prose | list | **Read** the **header** of a document. | - | prose | document(node-ref)! | mutates=false | undoable=false"#,
    r#"prose.document.outline | Outline | prose | command | prose | list | **Read** the outline of a document: its headings, in the order they stand. | - | prose | document(node-ref)! | mutates=false | undoable=false"#,
    r#"prose.document.properties | Properties | prose | command | prose | tag | **Say** what a document is. | - | prose | document(node-ref)!,author(text)?,subject(text)?,keywords(text)?,comments(text)?,category(text)?,last_modified_by(text)?,revision(number)?,custom(json)? | mutates=true | undoable=true"#,
    r#"prose.document.source.read | Read | prose | command | prose | list | **Read** the **sources** of a document. | - | prose | document(node-ref)! | mutates=false | undoable=false"#,
    r#"prose.document.source.remove | Remove | prose | command | prose | trash | **Take away** a **source** of a document. | - | prose | document(node-ref)!,tag(text)! | mutates=true | undoable=true"#,
    r#"prose.document.source.set | Set | prose | command | prose | tag | **Say** what a **source** is. | - | prose | document(node-ref)!,tag(text)?,kind(text)?,author(text)?,title(text)?,year(text)?,publisher(text)?,city(text)?,journal(text)?,volume(text)?,pages(text)?,url(text)? | mutates=true | undoable=true"#,
    r#"prose.document.stats | Stats | prose | command | prose | list | **Read** the **statistics** of a document. | - | prose | document(node-ref)! | mutates=false | undoable=false"#,
    r#"prose.document.theme | Theme | prose | command | prose | paint | **Say** what the **theme** of a document is. | - | prose | document(node-ref)!,major(text)?,minor(text)?,accent(text)? | mutates=true | undoable=true"#,
    r#"prose.document.title | Title | prose | command | prose | pencil | **Say** what a document is. | - | prose | document(node-ref)!,title(text)! | mutates=true | undoable=true"#,
    r#"prose.endnote.add | Add | prose | command | prose | plus | Put an **endnote** in a document. | - | prose | document(node-ref)!,text(text)! | mutates=true | undoable=true"#,
    r#"prose.equation.set | Set | prose | command | prose | pencil | **Set** the parts of an equation. | - | prose | block(node-ref)!,parts(json)! | mutates=true | undoable=true"#,
    r#"prose.find | Find | prose | command | prose | list | **Find** a pattern in a block. | - | prose | block(node-ref)!,what(text)!,wildcards(bool)?,case(bool)? | mutates=false | undoable=false"#,
    r#"prose.footnote.add | Add | prose | command | prose | plus | Put a **footnote** in a document. | - | prose | document(node-ref)!,text(text)! | mutates=true | undoable=true"#,
    r#"prose.index.mark | Mark | prose | command | prose | tag | **Mark** a block with an index **entry**. | - | prose | block(node-ref)!,entry(text)!,subentry(text)? | mutates=true | undoable=true"#,
    r#"prose.index.read | Read | prose | command | prose | list | **Read** the **index** of a document. | - | prose | document(node-ref)! | mutates=false | undoable=false"#,
    r#"prose.list.add | Add | prose | command | prose | plus | Put a **list** in a document. | - | prose | document(node-ref)!,kind(text)! | mutates=true | undoable=true"#,
    r#"prose.media.add | Add | prose | command | prose | plus | Put a **media** in a document: a file the document uses. | - | prose | document(node-ref)!,name(text)!,path(text)! | mutates=true | undoable=true"#,
    r#"prose.media.alt | Alt | prose | command | prose | pencil | **Say** what a picture is. | - | prose | media(node-ref)!,alt(text)! | mutates=true | undoable=true"#,
    r#"prose.media.decorative | Decorative | prose | command | prose | pencil | **Say** that a picture is **decorative**. | - | prose | media(node-ref)!,decorative(bool)? | mutates=true | undoable=true"#,
    r#"prose.media.placement | Placement | prose | command | prose | pencil | **Say** where a picture **sits**. | - | prose | media(node-ref)!,placement(text)? | mutates=true | undoable=true"#,
    r#"prose.merge | Merge | prose | command | prose | list | **Merge** the fields of a document with the rows of a data source. | - | prose | headers(json)?,rows(json)! | mutates=false | undoable=false"#,
    r#"prose.notes.convert | Convert | prose | command | prose | pencil | **Convert** the **notes** of a document. | - | prose | document(node-ref)!,to(text)! | mutates=true | undoable=true"#,
    r#"prose.number.format | Format | prose | command | prose | list | **Read** the **numbering format** of a number. | - | prose | value(number)!,format(text)! | mutates=false | undoable=false"#,
    r#"prose.page.background | Background | prose | command | prose | paint | **Paint** the page of a document with a **colour**. | - | prose | document(node-ref)!,colour(text)! | mutates=true | undoable=true"#,
    r#"prose.page.borders | Borders | prose | command | prose | rect | **Set** the **borders** of the page of a document. | - | prose | document(node-ref)!,kind(text)?,style(text)?,width(number)?,colour(text)?,sides(json)? | mutates=true | undoable=true"#,
    r#"prose.page.columns | Columns | prose | command | prose | rect | **Set** the **columns** of the page of a document. | - | prose | document(node-ref)!,count(number)!,space(number)?,separator(bool)? | mutates=true | undoable=true"#,
    r#"prose.page.get | Get | prose | command | prose | rect | **Read** the page of a document. | - | prose | document(node-ref)! | mutates=false | undoable=false"#,
    r#"prose.page.numbers | Numbers | prose | command | prose | list | **Set** the **numbering** of the pages. | - | prose | document(node-ref)!,format(text)?,start(number)? | mutates=true | undoable=true"#,
    r#"prose.page.set | Set | prose | command | prose | rect | **Set** the page of a document. | - | prose | document(node-ref)!,width(number)!,height(number)!,top(number)?,bottom(number)?,left(number)?,right(number)? | mutates=true | undoable=true"#,
    r#"prose.page.watermark | Watermark | prose | command | prose | paint | **Put** a **watermark** on the page of a document. | - | prose | document(node-ref)!,text(text)!,diagonal(bool)?,colour(text)? | mutates=true | undoable=true"#,
    r#"prose.replace | Replace | prose | command | prose | pencil | **Replace** a pattern in a block. | - | prose | block(node-ref)!,what(text)!,with(text)!,wildcards(bool)?,case(bool)? | mutates=true | undoable=true"#,
    r#"prose.revision.accept | Accept | prose | command | prose | check | **Accept** a revision. | - | prose | block(node-ref)!,run(number)! | mutates=true | undoable=true"#,
    r#"prose.revision.accept.all | All | prose | command | prose | check | **Accept** every revision of a block. | - | prose | block(node-ref)! | mutates=true | undoable=true"#,
    r#"prose.revision.reject | Reject | prose | command | prose | undo | **Reject** a revision. | - | prose | block(node-ref)!,run(number)! | mutates=true | undoable=true"#,
    r#"prose.revision.reject.all | All | prose | command | prose | undo | **Reject** every revision of a block. | - | prose | block(node-ref)! | mutates=true | undoable=true"#,
    r#"prose.run.format | Format | prose | command | prose | pencil | The **character** of a piece of text. | - | prose | block(node-ref)!,start(number)!,length(number)!,bold(bool)?,italic(bool)?,underline(bool)?,colour(text)?,size(number)? | mutates=true | undoable=true"#,
    r#"prose.style.add | Add | prose | command | prose | plus | Put a **style** in a document. | - | prose | document(node-ref)!,name(text)!,style_kind(text)?,based_on(node-ref)? | mutates=true | undoable=true"#,
    r#"prose.style.character | Character | prose | command | prose | pencil | **Set** the character of a style. | - | prose | style(node-ref)!,font(text)?,size(number)?,colour(text)?,bold(bool)?,italic(bool)?,underline(bool)? | mutates=true | undoable=true"#,
    r#"prose.style.character.read | Read | prose | command | prose | list | **Read** the character of a style, with the character of its base under it. | - | prose | style(node-ref)! | mutates=false | undoable=false"#,
    r#"prose.style.delete | Delete | prose | command | prose | minus | **Delete** a **style**. | - | prose | style(node-ref)! | mutates=true | undoable=true"#,
    r#"prose.style.level | Level | prose | command | prose | list | **Set** the level of a style — or take it away. | - | prose | style(node-ref)!,level(number)? | mutates=true | undoable=true"#,
    r#"prose.style.level.read | Read | prose | command | prose | list | **Read** the level of a style, with the level of its base under it. | - | prose | style(node-ref)! | mutates=false | undoable=false"#,
    r#"prose.style.list | List | prose | command | prose | list | **Read** the **styles** of a document. | - | prose | document(node-ref)! | mutates=false | undoable=false"#,
    r#"prose.style.paragraph | Paragraph | prose | command | prose | pencil | **Set** the paragraph of a style. | - | prose | style(node-ref)!,alignment(text)?,space_before(number)?,space_after(number)?,line_spacing(number)?,indent_left(number)?,indent_right(number)?,indent_first(number)? | mutates=true | undoable=true"#,
    r#"prose.style.paragraph.read | Read | prose | command | prose | list | **Read** the paragraph of a style, with the paragraph of its base under it. | - | prose | style(node-ref)! | mutates=false | undoable=false"#,
    r#"prose.table.alt | Alt | prose | command | prose | pencil | **Say** what a table is. | - | prose | table(node-ref)!,alt(text)! | mutates=true | undoable=true"#,
    r#"prose.table.cell.get | Get | prose | command | prose | list | **Read** a cell of a table. | - | prose | table(node-ref)!,row(number)!,column(number)! | mutates=false | undoable=false"#,
    r#"prose.table.cell.set | Set | prose | command | prose | pencil | **Set** a cell of a table. | - | prose | table(node-ref)!,row(number)!,column(number)!,value(json)! | mutates=true | undoable=true"#,
    r#"prose.table.column.add | Add | prose | command | prose | plus | Put a **column** in a table. | - | prose | table(node-ref)!,at(number)? | mutates=true | undoable=true"#,
    r#"prose.table.column.remove | Remove | prose | command | prose | minus | Take a **column** out of a table. | - | prose | table(node-ref)!,column(number)! | mutates=true | undoable=true"#,
    r#"prose.table.header | Header | prose | command | prose | list | **Say** whether the first row of a table is a header row. | - | prose | table(node-ref)!,header(bool)! | mutates=true | undoable=true"#,
    r#"prose.table.merge | Merge | prose | command | prose | link | **Merge** cells of a table into one. | - | prose | table(node-ref)!,row(number)!,column(number)!,rows(number)?,columns(number)? | mutates=true | undoable=true"#,
    r#"prose.table.merges | Merges | prose | command | prose | list | **Read** the **merged cells** of a table. | - | prose | table(node-ref)! | mutates=false | undoable=false"#,
    r#"prose.table.row.add | Add | prose | command | prose | plus | Put a **row** in a table. | - | prose | table(node-ref)!,at(number)? | mutates=true | undoable=true"#,
    r#"prose.table.row.remove | Remove | prose | command | prose | minus | Take a **row** out of a table. | - | prose | table(node-ref)!,row(number)! | mutates=true | undoable=true"#,
    r#"prose.table.split | Split | prose | command | prose | minus | **Split** a merged cell of a table. | - | prose | table(node-ref)!,row(number)!,column(number)! | mutates=true | undoable=true"#,
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
fn written(command: &CommandSpec) -> String {
    let params: Vec<String> = command
        .params
        .iter()
        .map(|param| {
            format!("{}({}){}", param.name, param.kind.json_name(), if param.required { "!" } else { "?" })
        })
        .collect();
    format!(
        "{} | {} | {} | {} | {} | {} | {} | {} | {} | {} | mutates={} | undoable={}",
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
        command.undoable
    )
}

/// The registry says what the snapshot says.
#[test]
fn the_registry_is_the_snapshot() {
    let commands = the_battery();
    assert!(!commands.is_empty(), "the battery has no commands");
    let found: Vec<String> = commands.iter().map(written).collect();
    if std::env::var("CRAFTKIT_DUMP").is_ok() {
        for line in &found {
            println!("    r#\"{line}\"#,");
        }
        return;
    }
    assert_eq!(found, EXPECTED, "the registry is not what it was");
}
