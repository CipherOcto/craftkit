//! The **pdf battery's registry**, pinned.
//!
//! # Why this test exists
//!
//! §867 converted this battery from one declaration per command to
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

#![cfg(feature = "pdf")]

use craftkit_sdk::command::CommandSpec;

/// The commands of the pdf battery, as the registry held them.
///
/// **The snapshot**: taken before the conversion, from the registry.
const EXPECTED: &[&str] = &[
    r#"pdf.annot.add | Add | pdf | command | pdf | plus | Put an **annotation** on a page. | - | pdf | page(node-ref)!,object(number)!,subtype(text)!,rect(json)!,contents(text)?,destination(node-ref)?,uri(text)? | mutates=true | undoable=true"#,
    r#"pdf.annot.appearance.write | Write | pdf | command | pdf | pencil | **Write** the **appearance** of an annotation. | - | pdf | annotation(node-ref)!,to(text)! | mutates=true | undoable=true"#,
    r#"pdf.annot.attachment.add | Add | pdf | command | pdf | plus | **Put** a **file attachment** on a page. | - | pdf | page(node-ref)!,object(number)!,at(json)!,name(text)!,path(text)!,to(text)!,icon(text)?,colour(json)? | mutates=true | undoable=true"#,
    r#"pdf.annot.callout.add | Add | pdf | command | pdf | pencil | **Put** a **callout** on a page. | - | pdf | page(node-ref)!,object(number)!,rect(json)!,knee(json)!,point(json)!,contents(text)!,size(number)? | mutates=true | undoable=true"#,
    r#"pdf.annot.caret.add | Add | pdf | command | pdf | pencil | **Put** a **caret** on a page. | - | pdf | page(node-ref)!,object(number)!,rect(json)!,colour(json)?,contents(text)? | mutates=true | undoable=true"#,
    r#"pdf.annot.customstamp.add | Add | pdf | command | pdf | pencil | **Put** a **custom stamp** on a page. | - | pdf | page(node-ref)!,object(number)!,rect(json)!,name(text)!,picture(node-ref)!,image(bool)?,size(json)? | mutates=true | undoable=true"#,
    r#"pdf.annot.flags | Flags | pdf | command | pdf | pencil | **Say** the **flags** of an annotation. | - | pdf | annotation(node-ref)!,print(bool)?,hidden(bool)? | mutates=true | undoable=true"#,
    r#"pdf.annot.freetext.add | Add | pdf | command | pdf | pencil | **Put** a **text box** on a page. | - | pdf | page(node-ref)!,object(number)!,rect(json)!,contents(text)!,size(number)?,colour(json)?,background(json)?,width(number)?,align(number)? | mutates=true | undoable=true"#,
    r#"pdf.annot.ink.add | Add | pdf | command | pdf | pencil | **Put** **ink** on a page. | - | pdf | page(node-ref)!,object(number)!,strokes(json)!,colour(json)?,width(number)? | mutates=true | undoable=true"#,
    r#"pdf.annot.line.add | Add | pdf | command | pdf | pencil | **Put** a **line** on a page. | - | pdf | page(node-ref)!,object(number)!,from(json)!,to(json)!,arrow(bool)?,colour(json)?,width(number)? | mutates=true | undoable=true"#,
    r#"pdf.annot.mark.add | Add | pdf | command | pdf | pencil | **Put** a **mark** on a page. | - | pdf | page(node-ref)!,object(number)!,rect(json)!,mark(text)!,colour(json)? | mutates=true | undoable=true"#,
    r#"pdf.annot.polygon.add | Add | pdf | command | pdf | pencil | **Put** a **polygon** on a page. | - | pdf | page(node-ref)!,object(number)!,kind(text)!,vertices(json)!,colour(json)?,width(number)?,fill(json)?,cloud(bool)? | mutates=true | undoable=true"#,
    r#"pdf.annot.redact.add | Add | pdf | command | pdf | pencil | **Put** a **mark of a redaction** on a page. | - | pdf | page(node-ref)!,object(number)!,region(json)!,text(text)?,colour(json)?,size(number)?,align(number)?,repeat(bool)? | mutates=true | undoable=true"#,
    r#"pdf.annot.stamp.add | Add | pdf | command | pdf | pencil | **Put** a **stamp** on a page. | - | pdf | page(node-ref)!,object(number)!,rect(json)!,kind(text)!,by(text)?,colour(json)? | mutates=true | undoable=true"#,
    r#"pdf.annot.typedsignature.add | Add | pdf | command | pdf | pencil | **Put** a **typed signature** on a page. | - | pdf | page(node-ref)!,object(number)!,rect(json)!,contours(json)!,colour(json)? | mutates=true | undoable=true"#,
    r#"pdf.attachments.read | Read | pdf | command | pdf | list | **Read** the **attachments** of a PDF file. | - | pdf | name(text)? | mutates=false | undoable=false"#,
    r#"pdf.comment.add | Add | pdf | command | pdf | plus | **Put** a **note** on a page. | - | pdf | page(node-ref)!,at(json)!,contents(text)?,author(text)?,icon(text)?,colour(text)?,date(text)?,to(text)! | mutates=true | undoable=true"#,
    r#"pdf.comment.flatten | Flatten | pdf | command | pdf | pencil | **Flatten** the comments of a file: they are drawn into the pages they are on, and taken away. | - | pdf | page(node-ref)? | mutates=true | undoable=true"#,
    r#"pdf.comment.info | Info | pdf | command | pdf | pencil | **Change** the **author**, the **subject** and the **icon** of a comment. | - | pdf | comment(node-ref)!,author(text)?,subject(text)?,icon(text)?,to(text)? | mutates=true | undoable=true"#,
    r#"pdf.comment.read | Read | pdf | command | pdf | list | **Read** the comments of a file. | - | pdf | page(node-ref)? | mutates=false | undoable=false"#,
    r#"pdf.compare | Compare | pdf | command | pdf | git-compare | **Compare** two files of PDF: the differences between their text. | - | pdf | old(text)!,new(text)! | mutates=false | undoable=false"#,
    r#"pdf.compare.mark | Mark | pdf | command | pdf | git-compare | **Mark** the differences between two files: the highlights over the text that changed, and the notes where text was deleted. | - | pdf | old(text)! | mutates=true | undoable=true"#,
    r#"pdf.content.operators | Operators | pdf | command | pdf | list | Read the **operators** of a content stream. | - | pdf | stream(node-ref)!,filter(text)? | mutates=false | undoable=false"#,
    r#"pdf.content.write | Write | pdf | command | pdf | file | Write the **content** of a stream back. | - | pdf | stream(node-ref)! | mutates=true | undoable=true"#,
    r#"pdf.convert | Convert | pdf | command | pdf | check | **Convert** a PDF file: fix what can be fixed, and say what is left. | - | pdf | level(text)? | mutates=true | undoable=true"#,
    r#"pdf.dests.read | Read | pdf | command | pdf | list | **Read** the **named destinations** of a PDF file. | - | pdf | name(text)? | mutates=false | undoable=false"#,
    r#"pdf.embeddedfile.add | Add | pdf | command | pdf | plus | **Name** a **file specification** in the document's tree of embedded files. | - | pdf | specification(node-ref)!,name(text)? | mutates=true | undoable=true"#,
    r#"pdf.encrypt.read | Read | pdf | command | pdf | list | **Read** the **encryption** of a PDF file. | - | pdf | key(text)? | mutates=false | undoable=false"#,
    r#"pdf.entry.remove | Remove | pdf | command | pdf | minus | **Take away** a key of an object's dictionary. | - | pdf | object(node-ref)!,key(text)! | mutates=true | undoable=true"#,
    r#"pdf.entry.set | Set | pdf | command | pdf | pencil | **Set** a key of an object's dictionary. | - | pdf | object(node-ref)!,key(text)!,value(json)! | mutates=true | undoable=true"#,
    r#"pdf.field.add | Add | pdf | command | pdf | plus | **Put** a **text field** on a page. | - | pdf | page(node-ref)!,object(number)!,rect(json)!,name(text)?,multiline(bool)?,value(text)?,appearance_to(text)? | mutates=true | undoable=true"#,
    r#"pdf.field.appearance | Appearance | pdf | command | pdf | list | **Read** the **appearance** of a field. | - | pdf | field(node-ref)! | mutates=false | undoable=false"#,
    r#"pdf.field.appearance.write | Write | pdf | command | pdf | pencil | **Write** the **appearance** of a field. | - | pdf | widget(node-ref)!,to(text)! | mutates=true | undoable=true"#,
    r#"pdf.field.button.add | Add | pdf | command | pdf | plus | **Put** a **push button** on a page. | - | pdf | page(node-ref)!,object(number)!,rect(json)!,name(text)?,caption(text)!,appearance_to(text)? | mutates=true | undoable=true"#,
    r#"pdf.field.calculate | Calculate | pdf | command | pdf | list | **Read** the **calculation** of a field. | - | pdf | field(node-ref)! | mutates=false | undoable=false"#,
    r#"pdf.field.checkbox.add | Add | pdf | command | pdf | plus | **Put** a **check box** on a page. | - | pdf | page(node-ref)!,object(number)!,rect(json)!,name(text)?,style(text)?,on_to(text)?,off_to(text)? | mutates=true | undoable=true"#,
    r#"pdf.field.children | Children | pdf | command | pdf | list | **Say** the **children** of a field. | - | pdf | field(node-ref)! | mutates=false | undoable=false"#,
    r#"pdf.field.combo.add | Add | pdf | command | pdf | plus | **Put** a **drop-down** on a page. | - | pdf | page(node-ref)!,object(number)!,rect(json)!,name(text)?,options(json)!,editable(bool)?,value(text)?,appearance_to(text)? | mutates=true | undoable=true"#,
    r#"pdf.field.date.add | Add | pdf | command | pdf | plus | **Put** a **date field** on a page. | - | pdf | page(node-ref)!,object(number)!,rect(json)!,name(text)?,value(text)?,appearance_to(text)? | mutates=true | undoable=true"#,
    r#"pdf.field.flags | Flags | pdf | command | pdf | pencil | **Say** how a field behaves. | - | pdf | field(node-ref)!,read_only(bool)?,required(bool)? | mutates=true | undoable=true"#,
    r#"pdf.field.format | Format | pdf | command | pdf | list | **Read** the **format** of a field. | - | pdf | field(node-ref)! | mutates=false | undoable=false"#,
    r#"pdf.field.image.add | Add | pdf | command | pdf | plus | **Put** an **image field** on a page. | - | pdf | page(node-ref)!,object(number)!,rect(json)!,name(text)?,appearance_to(text)? | mutates=true | undoable=true"#,
    r#"pdf.field.inherit | Inherit | pdf | command | pdf | link | **Say** what a field **inherits** from its parents. | - | pdf | field(node-ref)! | mutates=false | undoable=false"#,
    r#"pdf.field.list.add | Add | pdf | command | pdf | plus | **Put** a **list box** on a page. | - | pdf | page(node-ref)!,object(number)!,rect(json)!,name(text)?,options(json)!,multi(bool)?,value(text)?,appearance_to(text)? | mutates=true | undoable=true"#,
    r#"pdf.field.look.set | Set | pdf | command | pdf | pencil | **Set** the **look** of a field. | - | pdf | field(node-ref)!,border(json)?,fill(json)?,width(number)?,style(text)?,text_colour(json)?,font(text)?,font_size(number)?,check_style(text)?,rect(json)? | mutates=true | undoable=true"#,
    r#"pdf.field.parent | Parent | pdf | command | pdf | link | **Put** a field in a field. | - | pdf | field(node-ref)!,parent(node-ref)? | mutates=true | undoable=true"#,
    r#"pdf.field.properties.set | Set | pdf | command | pdf | pencil | **Set** the **properties** of a field. | - | pdf | field(node-ref)!,name(text)?,tooltip(text)?,max_len(number)?,options(json)?,quadding(number)?,default_value(text)?,locked(bool)?,multiline(bool)? | mutates=true | undoable=true"#,
    r#"pdf.field.radio.add | Add | pdf | command | pdf | plus | **Put** a **radio button** on a page. | - | pdf | page(node-ref)!,object(number)!,rect(json)!,group(text)?,export(text)!,style(text)?,on_to(text)?,off_to(text)? | mutates=true | undoable=true"#,
    r#"pdf.field.read | Read | pdf | command | pdf | list | **Read** the fields of a form. | - | pdf | field(node-ref)? | mutates=false | undoable=false"#,
    r#"pdf.field.set | Set | pdf | command | pdf | pencil | **Fill** a field of a form. | - | pdf | field(node-ref)!,value(text)?,values(json)? | mutates=true | undoable=true"#,
    r#"pdf.field.signature.add | Add | pdf | command | pdf | plus | **Put** a **signature field** on a page. | - | pdf | page(node-ref)!,object(number)!,rect(json)!,name(text)?,appearance_to(text)? | mutates=true | undoable=true"#,
    r#"pdf.field.validate | Validate | pdf | command | pdf | list | **Read** the **validation** of a field. | - | pdf | field(node-ref)! | mutates=false | undoable=false"#,
    r#"pdf.field.value | Value | pdf | command | pdf | list | **Read** the **value** a field is **calculated** to hold. | - | pdf | field(node-ref)! | mutates=false | undoable=false"#,
    r#"pdf.field.widgets | Widgets | pdf | command | pdf | list | **Say** the **widgets** of a field. | - | pdf | field(node-ref)! | mutates=false | undoable=false"#,
    r#"pdf.figure.alt.set | Set | pdf | command | pdf | pencil | **Set** the **alternative text** of a figure. | - | pdf | figure(node-ref)!,alt(text)! | mutates=true | undoable=true"#,
    r#"pdf.figure.read | Read | pdf | command | pdf | list | The **figures** of a file. | - | pdf |  | mutates=false | undoable=false"#,
    r#"pdf.file.decrypt | Decrypt | pdf | command | pdf | file | **Decrypt** a file and **write** it out without its encryption. | - | pdf | password(text)!,path(text)! | mutates=false | undoable=false"#,
    r#"pdf.file.encrypt | Encrypt | pdf | command | pdf | file | **Encrypt** a file with a password, and **write** it out. | - | pdf | password(text)!,path(text)!,owner(text)?,cipher(text)? | mutates=false | undoable=false"#,
    r#"pdf.file.free | Free | pdf | command | pdf | list | The **free numbers** of the file. | - | pdf | after(number)? | mutates=false | undoable=false"#,
    r#"pdf.file.load | Load | pdf | command | pdf | file | Load a PDF **file** into the document. | - | pdf | path(text)!,into(text)! | mutates=true | undoable=true"#,
    r#"pdf.file.write | Write | pdf | command | pdf | file | Write the document out as a PDF **file**. | - | pdf | path(text)!,root(number)? | mutates=true | undoable=true"#,
    r#"pdf.file.xref | Xref | pdf | command | pdf | list | Read the **cross-reference table** of a file. | - | pdf | path(text)! | mutates=false | undoable=false"#,
    r#"pdf.font.unicode | Unicode | pdf | command | pdf | list | **Read** the **Unicode map** of a font. | - | pdf | font(node-ref)! | mutates=false | undoable=false"#,
    r#"pdf.fonts | Fonts | pdf | command | pdf | list | **Read** the fonts of a PDF file. | - | pdf | font(node-ref)? | mutates=false | undoable=false"#,
    r#"pdf.form.data.export | Export | pdf | command | pdf | file | **Write** the **data of a form** as a file. | - | pdf | path(text)!,format(text)?,file(text)? | mutates=true | undoable=true"#,
    r#"pdf.form.data.import | Import | pdf | command | pdf | file | **Read** the **data of a form** from a file, and write it into the form. | - | pdf | path(text)! | mutates=true | undoable=true"#,
    r#"pdf.form.flatten | Flatten | pdf | command | pdf | pencil | **Flatten** the form of a file: the fields are drawn into the pages they are on, and taken away. | - | pdf | page(node-ref)? | mutates=true | undoable=true"#,
    r#"pdf.form.read | Read | pdf | command | pdf | list | **Read** the **form** of a PDF file. | - | pdf | key(text)? | mutates=false | undoable=false"#,
    r#"pdf.hidden.remove | Remove | pdf | command | pdf | remove | The **hidden information** of a file is taken away. | - | pdf | category(text)! | mutates=true | undoable=true"#,
    r#"pdf.hidden.scan | Scan | pdf | command | pdf | list | The **hidden information** of a file is counted. | - | pdf | category(text)? | mutates=false | undoable=false"#,
    r#"pdf.info.read | Read | pdf | command | pdf | list | **Read** the **information** of a PDF file. | - | pdf | key(text)? | mutates=false | undoable=false"#,
    r#"pdf.info.set | Set | pdf | command | pdf | pencil | Say which object is the file's **information**. | - | pdf | object(node-ref)! | mutates=true | undoable=true"#,
    r#"pdf.intent.write | Write | pdf | command | pdf | file | **Write** the **output intent** of a file. | - | pdf | path(text)! | mutates=true | undoable=true"#,
    r#"pdf.layers.read | Read | pdf | command | pdf | list | **Read** the **layers** of a PDF file. | - | pdf | name(text)? | mutates=false | undoable=false"#,
    r#"pdf.marked.read | Read | pdf | command | pdf | list | **Read** the **marked content** of a PDF file: the text under every element of the structure tree. | - | pdf | kind(text)? | mutates=false | undoable=false"#,
    r#"pdf.markup.add | Add | pdf | command | pdf | pencil | **Mark** the text of a page: a highlight, an underline, a strikeout or a squiggly underline. | - | pdf | page(node-ref)!,object(number)!,kind(text)!,region(json)! | mutates=true | undoable=true"#,
    r#"pdf.metadata.write | Write | pdf | command | pdf | file | **Write** the **XMP metadata** of a file. | - | pdf | path(text)!,level(text)! | mutates=true | undoable=true"#,
    r#"pdf.object.add | Add | pdf | command | pdf | plus | Put an indirect object in the document. | - | pdf | object(number)!,entries(json)! | mutates=true | undoable=true"#,
    r#"pdf.object.number | Number | pdf | command | pdf | hash | The **number** of an object. | - | pdf | object(node-ref)!,number(number)! | mutates=true | undoable=true"#,
    r#"pdf.object.remove | Remove | pdf | command | pdf | trash | Take an **indirect object** away. | - | pdf | object(node-ref)! | mutates=true | undoable=true"#,
    r#"pdf.objstm.read | Read | pdf | command | pdf | list | Read the **objects** of an object stream. | - | pdf | stream(node-ref)! | mutates=false | undoable=false"#,
    r#"pdf.outline.add | Add | pdf | command | pdf | plus | Put a **bookmark** in the outline. | - | pdf | title(text)!,destination(node-ref)?,parent(node-ref)?,object(number)? | mutates=true | undoable=true"#,
    r#"pdf.outline.read | Read | pdf | command | pdf | list | Read the **outline**: the bookmarks, in the order they are in. | - | pdf | root(node-ref)? | mutates=false | undoable=false"#,
    r#"pdf.page.add | Add | pdf | command | pdf | plus | Put a **page** in the document. | - | pdf | object(number)!,entries(json)?,parent(node-ref)? | mutates=true | undoable=true"#,
    r#"pdf.page.annotations | Annotations | pdf | command | pdf | list | The **annotations** of a page. | - | pdf | page(node-ref)! | mutates=false | undoable=false"#,
    r#"pdf.page.attributes | Attributes | pdf | command | pdf | list | The **attributes** of a page, and the ones it inherits. | - | pdf | page(node-ref)! | mutates=false | undoable=false"#,
    r#"pdf.page.background | Background | pdf | command | pdf | pencil | **Draw** a background on a page: a colour behind the content of it. | - | pdf | page(node-ref)!,colour(text)?,opacity(number)? | mutates=true | undoable=true"#,
    r#"pdf.page.box | Box | pdf | command | pdf | pencil | Set a **box** of a page. | - | pdf | page(node-ref)!,which(text)!,rect(json)! | mutates=true | undoable=true"#,
    r#"pdf.page.boxes | Boxes | pdf | command | pdf | list | The **boxes** of a page. | - | pdf | page(node-ref)! | mutates=false | undoable=false"#,
    r#"pdf.page.fonts | Fonts | pdf | command | pdf | list | **Read** the **fonts** of a page: the fonts it names, and the maps they carry. | - | pdf | page(node-ref)! | mutates=false | undoable=false"#,
    r#"pdf.page.footer | Footer | pdf | command | pdf | pencil | **Draw** a footer on a page: the text at the bottom of it. | - | pdf | page(node-ref)!,left(text)?,centre(text)?,right(text)?,size(number)?,colour(text)?,underline(bool)?,margins(json)?,start_number(number)?,date(json)? | mutates=true | undoable=true"#,
    r#"pdf.page.header | Header | pdf | command | pdf | pencil | **Draw** a header on a page: the text at the top of it. | - | pdf | page(node-ref)!,left(text)?,centre(text)?,right(text)?,size(number)?,colour(text)?,underline(bool)?,margins(json)?,start_number(number)?,date(json)? | mutates=true | undoable=true"#,
    r#"pdf.page.labels | Labels | pdf | command | pdf | pencil | Give the **pages** of a document their labels. | - | pdf | pages(node-ref)!,labels(json)! | mutates=true | undoable=true"#,
    r#"pdf.page.marks.remove | Remove | pdf | command | pdf | trash | **Take away** the marks of a page: the watermark, the header, the footer or the background. | - | pdf | page(node-ref)!,kind(text)! | mutates=true | undoable=true"#,
    r#"pdf.page.remove | Remove | pdf | command | pdf | trash | Take a **page** away. | - | pdf | page(node-ref)! | mutates=true | undoable=true"#,
    r#"pdf.page.rotate | Rotate | pdf | command | pdf | rotate | Turn a **page**. | - | pdf | page(node-ref)!,degrees(number)! | mutates=true | undoable=true"#,
    r#"pdf.page.rotation | Rotation | pdf | command | pdf | rotate | The **rotation** of a page, as it stands. | - | pdf | page(node-ref)! | mutates=false | undoable=false"#,
    r#"pdf.page.tabs | Tabs | pdf | command | pdf | list | **Say** the **tab order** of a page. | - | pdf | page(node-ref)!,tabs(text)? | mutates=true | undoable=true"#,
    r#"pdf.page.text | Text | pdf | command | pdf | list | **Read** the **text** of a page. | - | pdf | page(node-ref)! | mutates=false | undoable=false"#,
    r#"pdf.page.text.hidden | Hidden | pdf | command | pdf | list | The **text of a page that is not seen**. | - | pdf | page(node-ref)! | mutates=false | undoable=false"#,
    r#"pdf.page.text.under | Under | pdf | command | pdf | list | The **text** of a page that is **under a region**. | - | pdf | page(node-ref)!,rect(json)! | mutates=false | undoable=false"#,
    r#"pdf.page.transition | Transition | pdf | command | pdf | list | **Read** the **transition** of a page. | - | pdf | page(node-ref)! | mutates=false | undoable=false"#,
    r#"pdf.page.watermark | Watermark | pdf | command | pdf | pencil | **Draw** a watermark on a page: a text, turned about the centre of the page, in the tree's own grey. | - | pdf | page(node-ref)!,text(text)!,size(number)?,rotation(number)?,opacity(number)?,colour(text)?,behind(bool)? | mutates=true | undoable=true"#,
    r#"pdf.preflight | Preflight | pdf | command | pdf | check | **Check** a PDF file: what it declares and what it breaks. | - | pdf | level(text)? | mutates=false | undoable=false"#,
    r#"pdf.redact.annotations | Annotations | pdf | command | pdf | remove | The **annotations under a region are taken away**. | - | pdf | page(node-ref)!,rect(json)! | mutates=true | undoable=true"#,
    r#"pdf.redact.box | Box | pdf | command | pdf | remove | The **box** of a redaction is drawn into the page. | - | pdf | page(node-ref)!,rect(json)!,color(json)? | mutates=true | undoable=true"#,
    r#"pdf.redact.drawings | Drawings | pdf | command | pdf | remove | The **drawings under a region are taken away**. | - | pdf | page(node-ref)!,rect(json)! | mutates=true | undoable=true"#,
    r#"pdf.redact.pattern.find | Find | pdf | command | pdf | find | The **text** that matches a pattern: the ranges of it, and the text of each range. | - | pdf | text(text)!,pattern(text)! | mutates=false | undoable=false"#,
    r#"pdf.redact.patterns | Patterns | pdf | command | pdf | list | The **patterns** of a redaction: the tree's five, with their labels. | - | pdf |  | mutates=false | undoable=false"#,
    r#"pdf.redact.text | Text | pdf | command | pdf | remove | The **text under a region is removed**. | - | pdf | page(node-ref)!,rect(json)! | mutates=true | undoable=true"#,
    r#"pdf.ref.add | Add | pdf | command | pdf | link | Say that one object refers to another. | - | pdf | from(node-ref)!,key(text)!,to(node-ref)!,generation(number)?,list(bool)? | mutates=true | undoable=true"#,
    r#"pdf.refs | Refs | pdf | command | pdf | list | The references of the document. | - | pdf |  | mutates=false | undoable=false"#,
    r#"pdf.scripts.read | Read | pdf | command | pdf | list | **Read** the **JavaScript** of a PDF file. | - | pdf | name(text)? | mutates=false | undoable=false"#,
    r#"pdf.signatures.read | Read | pdf | command | pdf | list | **Read** the **signatures** of a PDF file. | - | pdf | field(text)? | mutates=false | undoable=false"#,
    r#"pdf.stream.add | Add | pdf | command | pdf | plus | Put an indirect object that has a **stream** in the document. | - | pdf | object(number)!,entries(json)!,stream(text)! | mutates=true | undoable=true"#,
    r#"pdf.stream.decode | Decode | pdf | command | pdf | file | **Decode** a stream's bytes. | - | PDF/Streams | stream(node-ref)!,filter(text)?,predictor(number)?,to(text)! | mutates=false | undoable=false"#,
    r#"pdf.structure.read | Read | pdf | command | pdf | list | **Read** the **structure** of a PDF file. | - | pdf |  | mutates=false | undoable=false"#,
    r#"pdf.tab.order | Order | pdf | command | pdf | list | **Read** the **tab order** of a form. | - | pdf | page(node-ref)? | mutates=false | undoable=false"#,
    r#"pdf.trailer.read | Read | pdf | command | pdf | list | **Read** the trailer of a PDF file. | - | pdf | key(text)? | mutates=false | undoable=false"#,
];

/// The commands of the pdf battery, in the order of their ids.
fn the_battery() -> Vec<CommandSpec> {
    let ecosystem = craftkit_studio::ecosystem().expect("the ecosystem is not a mess");
    let mut commands: Vec<CommandSpec> =
        ecosystem.commands.all().iter().filter(|command| command.area == "pdf").cloned().collect();
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
