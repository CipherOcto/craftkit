//! The **layout battery's registry**, pinned.
//!
//! # Why this test exists
//!
//! §864 converted this battery from one declaration per command to
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

#![cfg(feature = "layout")]

use craftkit_sdk::command::CommandSpec;

/// The commands of the layout battery, as the registry held them.
///
/// **The snapshot**: taken before the conversion, from the registry.
const EXPECTED: &[&str] = &[
    r#"layout.article.add | Add | layout | command | layout | plus | **Add** items to an article, in the order they are given. | - | layout | article(node-ref)!,items(json)? | mutates=true | undoable=true"#,
    r#"layout.article.delete | Delete | layout | command | layout | minus | **Delete** an article. | - | layout | article(node-ref)! | mutates=true | undoable=true"#,
    r#"layout.article.new | New | layout | command | layout | plus | **New** article in a document. | - | layout | document(node-ref)!,name(text)?,items(json)? | mutates=true | undoable=true"#,
    r#"layout.article.options | Options | layout | command | layout | pencil | The **options** of an article: its name, whether it is exported and the order of its items. | - | layout | article(node-ref)!,to(text)?,export(bool)?,order(json)? | mutates=true | undoable=true"#,
    r#"layout.article.remove | Remove | layout | command | layout | minus | **Take** items out of an article. | - | layout | article(node-ref)!,items(json)? | mutates=true | undoable=true"#,
    r#"layout.asset.add | Add | layout | command | layout | plus | Put an **asset** in a document: a file the document places. | - | layout | document(node-ref)!,name(text)!,path(text)!,kind(text)? | mutates=true | undoable=true"#,
    r#"layout.bookmark.add | Add | layout | command | layout | tag | **Add** a bookmark to a document. | - | layout | document(node-ref)!,name(text)?,page(number)? | mutates=true | undoable=true"#,
    r#"layout.bookmark.delete | Delete | layout | command | layout | minus | **Delete** a bookmark. | - | layout | bookmark(node-ref)! | mutates=true | undoable=true"#,
    r#"layout.bookmark.rename | Rename | layout | command | layout | pencil | **Rename** a bookmark. | - | layout | bookmark(node-ref)!,name(text)! | mutates=true | undoable=true"#,
    r#"layout.document.add | Add | layout | command | layout | plus | Put a **document** in the document. | - | layout | name(text)! | mutates=true | undoable=true"#,
    r#"layout.document.articles | Articles | layout | command | layout | list | The **articles** of a document: their names, their items and whether they are exported. | - | layout | document(node-ref)! | mutates=false | undoable=false"#,
    r#"layout.document.bleed | Bleed | layout | command | layout | rect | **Set** the bleed of a document. | - | layout | document(node-ref)!,top(number)?,bottom(number)?,left(number)?,right(number)? | mutates=true | undoable=true"#,
    r#"layout.document.bookmarks | Bookmarks | layout | command | layout | list | The **bookmarks** of a document: the names and the pages they name. | - | layout | document(node-ref)! | mutates=false | undoable=false"#,
    r#"layout.document.links | Links | layout | command | layout | list | The **links** of the stories of a document, and which of them are out of date. | - | layout | document(node-ref)! | mutates=false | undoable=false"#,
    r#"layout.guide.add | Add | layout | command | layout | ruler | **Add** a guide to a page. | - | layout | page(node-ref)?,spread(node-ref)?,orientation(text)!,position(number)?,x(number)?,y(number)?,at(number)?,spread_guide(bool)?,liquid(bool)? | mutates=true | undoable=true"#,
    r#"layout.guide.create | Create | layout | command | layout | ruler | **Create** the guides of a page: a grid of them, across the page or across its margins. | - | layout | spread(node-ref)!,page(node-ref)?,rows(number)?,columns(number)?,row_gutter(number)?,column_gutter(number)?,fit_to(text)?,remove_existing(bool)? | mutates=true | undoable=true"#,
    r#"layout.guide.delete | Delete | layout | command | layout | minus | **Delete** a guide. | - | layout | guide(node-ref)! | mutates=true | undoable=true"#,
    r#"layout.guide.delete.all | All | layout | command | layout | minus | **Delete** every guide of a spread. | - | layout | spread(node-ref)! | mutates=true | undoable=true"#,
    r#"layout.guide.move | Move | layout | command | layout | ruler | **Move** a guide to another place. | - | layout | guide(node-ref)!,position(number)! | mutates=true | undoable=true"#,
    r#"layout.item.add | Add | layout | command | layout | plus | Put an **item** in a spread. | - | layout | spread(node-ref)!,name(text)!,kind(text)?,shape(text)?,rect(json)? | mutates=true | undoable=true"#,
    r#"layout.item.back | Back | layout | command | layout | arrows | **Send** an item to the **back**. | - | layout | item(node-ref)! | mutates=true | undoable=true"#,
    r#"layout.item.backward | Backward | layout | command | layout | arrows | **Move** an item **one step** towards the back. | - | layout | item(node-ref)! | mutates=true | undoable=true"#,
    r#"layout.item.fill | Fill | layout | command | layout | paint | **Fill** an item with a colour — or take the fill away. | - | layout | item(node-ref)!,colour(text)? | mutates=true | undoable=true"#,
    r#"layout.item.forward | Forward | layout | command | layout | arrows | **Move** an item **one step** towards the front. | - | layout | item(node-ref)! | mutates=true | undoable=true"#,
    r#"layout.item.front | Front | layout | command | layout | arrows | **Bring** an item to the **front**. | - | layout | item(node-ref)! | mutates=true | undoable=true"#,
    r#"layout.item.group | Group | layout | command | layout | group | **Group** items. | - | layout | items(json)!,name(text)? | mutates=true | undoable=true"#,
    r#"layout.item.hide | Hide | layout | command | layout | minus | **Hide** an item. | - | layout | item(node-ref)! | mutates=true | undoable=true"#,
    r#"layout.item.layer | Layer | layout | command | layout | plus | Put an item **on a layer**. | - | layout | item(node-ref)!,layer(node-ref)! | mutates=true | undoable=true"#,
    r#"layout.item.link | Link | layout | command | layout | link | **Set** the link of an item: a target outside the document. | - | layout | item(node-ref)!,target(text)! | mutates=true | undoable=true"#,
    r#"layout.item.list | List | layout | command | layout | list | **Read** the items of a spread, in the order they are drawn. | - | layout | spread(node-ref)! | mutates=false | undoable=false"#,
    r#"layout.item.lock | Lock | layout | command | layout | lock | **Lock** an item. | - | layout | item(node-ref)! | mutates=true | undoable=true"#,
    r#"layout.item.opacity | Opacity | layout | command | layout | paint | **Set** the opacity of an item. | - | layout | item(node-ref)!,opacity(number)! | mutates=true | undoable=true"#,
    r#"layout.item.page.link | Link | layout | command | layout | link | **Set** the link of an item to a **page** of the document. | - | layout | item(node-ref)!,page(node-ref)? | mutates=true | undoable=true"#,
    r#"layout.item.radius | Radius | layout | command | layout | rect | **Round** the corners of an item. | - | layout | item(node-ref)!,radius(number)! | mutates=true | undoable=true"#,
    r#"layout.item.rotate | Rotate | layout | command | layout | rect | **Turn** an item. | - | layout | item(node-ref)!,degrees(number)! | mutates=true | undoable=true"#,
    r#"layout.item.scale | Scale | layout | command | layout | rect | **Scale** an item. | - | layout | item(node-ref)!,factor(number)! | mutates=true | undoable=true"#,
    r#"layout.item.shadow | Shadow | layout | command | layout | paint | **Give** an item a shadow. | - | layout | item(node-ref)!,angle(number)!,distance(number)!,blur(number)?,colour(text)?,opacity(number)? | mutates=true | undoable=true"#,
    r#"layout.item.show | Show | layout | command | layout | plus | **Show** an item. | - | layout | item(node-ref)! | mutates=true | undoable=true"#,
    r#"layout.item.story | Story | layout | command | layout | plus | Put a **story** in an item: the text the frame shows. | - | layout | item(node-ref)!,story(node-ref)! | mutates=true | undoable=true"#,
    r#"layout.item.stroke | Stroke | layout | command | layout | paint | **Stroke** an item with a colour — or take the stroke away. | - | layout | item(node-ref)!,colour(text)?,weight(number)? | mutates=true | undoable=true"#,
    r#"layout.item.style | Style | layout | command | layout | plus | **Put** an item on a style — or take it off. | - | layout | item(node-ref)!,style(node-ref)? | mutates=true | undoable=true"#,
    r#"layout.item.ungroup | Ungroup | layout | command | layout | minus | **Ungroup** a group: the items go back where the group stood. | - | layout | group(node-ref)! | mutates=true | undoable=true"#,
    r#"layout.item.unlock | Unlock | layout | command | layout | lock | **Unlock** an item. | - | layout | item(node-ref)! | mutates=true | undoable=true"#,
    r#"layout.layer.add | Add | layout | command | layout | plus | Put a **layer** in a document. | - | layout | document(node-ref)!,name(text)!,color(text)? | mutates=true | undoable=true"#,
    r#"layout.layer.delete | Delete | layout | command | layout | minus | **Delete** a layer. | - | layout | layer(node-ref)! | mutates=true | undoable=true"#,
    r#"layout.layer.hide | Hide | layout | command | layout | minus | **Hide** a layer: what is on it is not drawn. | - | layout | layer(node-ref)! | mutates=true | undoable=true"#,
    r#"layout.layer.lock | Lock | layout | command | layout | lock | **Lock** a layer: what is on it may not be edited. | - | layout | layer(node-ref)! | mutates=true | undoable=true"#,
    r#"layout.layer.merge | Merge | layout | command | layout | group | **Merge** layers: the items of them move to one, and the others are gone. | - | layout | layers(json)!,into(node-ref)? | mutates=true | undoable=true"#,
    r#"layout.layer.move | Move | layout | command | layout | arrows | **Move** a layer in the order of the layers. | - | layout | layer(node-ref)!,to(number)! | mutates=true | undoable=true"#,
    r#"layout.layer.rename | Rename | layout | command | layout | pencil | **Rename** a layer. | - | layout | layer(node-ref)!,name(text)! | mutates=true | undoable=true"#,
    r#"layout.layer.show | Show | layout | command | layout | plus | **Show** a layer: what is on it is drawn again. | - | layout | layer(node-ref)! | mutates=true | undoable=true"#,
    r#"layout.layer.unlock | Unlock | layout | command | layout | lock | **Unlock** a layer: what is on it may be edited again. | - | layout | layer(node-ref)! | mutates=true | undoable=true"#,
    r#"layout.layer.unused.delete | Delete | layout | command | layout | minus | **Delete** the layers of a document that have **nothing on them**. | - | layout | document(node-ref)! | mutates=true | undoable=true"#,
    r#"layout.links | Links | layout | command | layout | link | **Read** the links of a document. | - | layout | document(node-ref)! | mutates=false | undoable=false"#,
    r#"layout.page.add | Add | layout | command | layout | plus | Put a **page** in a spread. | - | layout | spread(node-ref)!,name(text)!,width(number)?,height(number)? | mutates=true | undoable=true"#,
    r#"layout.page.columns | Columns | layout | command | layout | rect | **Set** the columns of a page. | - | layout | page(node-ref)!,count(number)!,gutter(number)? | mutates=true | undoable=true"#,
    r#"layout.page.delete | Delete | layout | command | layout | minus | **Delete** a page. | - | layout | page(node-ref)! | mutates=true | undoable=true"#,
    r#"layout.page.facing | Facing | layout | command | layout | page | **Read** the facing page of a page. | - | layout | page(node-ref)! | mutates=false | undoable=false"#,
    r#"layout.page.guides | Guides | layout | command | layout | list | The **guides** of a page: what they are and where they stand. | - | layout | page(node-ref)! | mutates=false | undoable=false"#,
    r#"layout.page.margin | Margin | layout | command | layout | rect | **Set** the margins of a page. | - | layout | page(node-ref)!,top(number)?,bottom(number)?,left(number)?,right(number)? | mutates=true | undoable=true"#,
    r#"layout.page.move | Move | layout | command | layout | arrows | **Move** a page to another place among the pages of its spread. | - | layout | page(node-ref)!,to(number)! | mutates=true | undoable=true"#,
    r#"layout.page.parent | Parent | layout | command | layout | page | **Give** a page a parent: the page it takes what it has not got of its own from. | - | layout | page(node-ref)!,parent(node-ref)! | mutates=true | undoable=true"#,
    r#"layout.page.parent.clear | Clear | layout | command | layout | page | Take the **parent** of a page away. | - | layout | page(node-ref)! | mutates=true | undoable=true"#,
    r#"layout.page.side | Side | layout | command | layout | page | **Say** which side of its spread a page is on. | - | layout | page(node-ref)!,side(text)! | mutates=true | undoable=true"#,
    r#"layout.spread.add | Add | layout | command | layout | plus | Put a **spread** in a document. | - | layout | document(node-ref)!,name(text)! | mutates=true | undoable=true"#,
    r#"layout.story.add | Add | layout | command | layout | plus | Put a **story** in a document: the text, stored once. | - | layout | document(node-ref)!,name(text)!,text(text)! | mutates=true | undoable=true"#,
    r#"layout.story.link | Link | layout | command | layout | link | **Link** a story to another story. | - | layout | story(node-ref)!,parent(node-ref)! | mutates=true | undoable=true"#,
    r#"layout.story.set | Set | layout | command | layout | pencil | **Set** the text of a story. | - | layout | story(node-ref)!,text(text)! | mutates=true | undoable=true"#,
    r#"layout.story.unlink | Unlink | layout | command | layout | minus | **Unlink** a story. | - | layout | story(node-ref)! | mutates=true | undoable=true"#,
    r#"layout.story.update | Update | layout | command | layout | arrow | **Update** the links of the stories: a story takes the content of its parent again, and its own edits are replaced. | - | layout | story(node-ref)? | mutates=true | undoable=true"#,
    r#"layout.style.add | Add | layout | command | layout | plus | **Add** a style to a document. | - | layout | document(node-ref)!,name(text)!,based_on(node-ref)? | mutates=true | undoable=true"#,
    r#"layout.style.fill | Fill | layout | command | layout | paint | **Fill** a style with a colour — or take the fill away. | - | layout | style(node-ref)!,colour(text)? | mutates=true | undoable=true"#,
    r#"layout.style.stroke | Stroke | layout | command | layout | paint | **Stroke** a style with a colour — or take the stroke away. | - | layout | style(node-ref)!,colour(text)?,weight(number)? | mutates=true | undoable=true"#,
];

/// The commands of the layout battery, in the order of their ids.
fn the_battery() -> Vec<CommandSpec> {
    let ecosystem = craftkit_studio::ecosystem().expect("the ecosystem is not a mess");
    let mut commands: Vec<CommandSpec> =
        ecosystem.commands.all().iter().filter(|command| command.area == "layout").cloned().collect();
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
