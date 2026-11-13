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
    r##"layout.article.add | Add | layout | command | layout | plus | **Add** items to an article, in the order they are given. | - | layout | article(node-ref)! help="The article.",items(json)? help="The items." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"layout.article.delete | Delete | layout | command | layout | minus | **Delete** an article. | - | layout | article(node-ref)! help="The article." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"layout.article.new | New | layout | command | layout | plus | **New** article in a document. | - | layout | document(node-ref)! help="The document.",name(text)? help="The name of the article.",items(json)? help="The items it holds, in the order they are in." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"layout.article.options | Options | layout | command | layout | pencil | The **options** of an article: its name, whether it is exported and the order of its items. | - | layout | article(node-ref)! help="The article.",to(text)? help="Its new name.",export(bool)? help="Whether it is exported.",order(json)? help="The order of its items: these come first, and the rest follow." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"layout.article.remove | Remove | layout | command | layout | minus | **Take** items out of an article. | - | layout | article(node-ref)! help="The article.",items(json)? help="The items." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"layout.asset.add | Add | layout | command | layout | plus | Put an **asset** in a document: a file the document places. | - | layout | document(node-ref)! help="The document it goes in.",name(text)! help="What to call it.",path(text)! help="Where the file is, relative to the document.",kind(text)? help="What kind of file it is: `image`, `eps`, `font`, …" | mutates=true | undoable=true | journal=true | example=false"##,
    r##"layout.bookmark.add | Add | layout | command | layout | tag | **Add** a bookmark to a document. | - | layout | document(node-ref)! help="The document.",name(text)? help="The name of the bookmark.",page(number)? help="The page it names, counted from one." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"layout.bookmark.delete | Delete | layout | command | layout | minus | **Delete** a bookmark. | - | layout | bookmark(node-ref)! help="The bookmark." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"layout.bookmark.rename | Rename | layout | command | layout | pencil | **Rename** a bookmark. | - | layout | bookmark(node-ref)! help="The bookmark.",name(text)! help="Its new name." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"layout.document.add | Add | layout | command | layout | plus | Put a **document** in the document. | - | layout | name(text)! help="What to call it." | mutates=true | undoable=true | journal=true | example=true"##,
    r##"layout.document.articles | Articles | layout | command | layout | list | The **articles** of a document: their names, their items and whether they are exported. | - | layout | document(node-ref)! help="The document." | mutates=false | undoable=false | journal=true | example=false"##,
    r##"layout.document.bleed | Bleed | layout | command | layout | rect | **Set** the bleed of a document. | - | layout | document(node-ref)! help="The document.",top(number)? help="The room at the top of the page.",bottom(number)? help="The room at the bottom of the page.",left(number)? help="The room at the left of the page.",right(number)? help="The room at the right of the page." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"layout.document.bookmarks | Bookmarks | layout | command | layout | list | The **bookmarks** of a document: the names and the pages they name. | - | layout | document(node-ref)! help="The document." | mutates=false | undoable=false | journal=true | example=false"##,
    r##"layout.document.links | Links | layout | command | layout | list | The **links** of the stories of a document, and which of them are out of date. | - | layout | document(node-ref)! help="The document." | mutates=false | undoable=false | journal=true | example=false"##,
    r##"layout.guide.add | Add | layout | command | layout | ruler | **Add** a guide to a page. | - | layout | page(node-ref)? help="The page it goes on.",spread(node-ref)? help="The spread it goes on, when the page is not named.",orientation(text)! help="`horizontal`, `vertical` or `both`.",position(number)? help="Where it stands.",x(number)? help="The x of a guide in **both** directions.",y(number)? help="The y of a guide in **both** directions.",at(number)? help="The x of a horizontal guide, which is where the page is found.",spread_guide(bool)? help="Whether it crosses the pasteboard.",liquid(bool)? help="Whether it is a liquid guide." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"layout.guide.create | Create | layout | command | layout | ruler | **Create** the guides of a page: a grid of them, across the page or across its margins. | - | layout | spread(node-ref)! help="The spread the pages are in.",page(node-ref)? help="The page, when it is not every page of the spread.",rows(number)? help="The rows the page is cut into.",columns(number)? help="The columns the page is cut into.",row_gutter(number)? help="The room between the rows.",column_gutter(number)? help="The room between the columns.",fit_to(text)? help="`margins` — the default — or `page`.",remove_existing(bool)? help="Whether the guides that are there are cleared first." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"layout.guide.delete | Delete | layout | command | layout | minus | **Delete** a guide. | - | layout | guide(node-ref)! help="The guide." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"layout.guide.delete.all | All | layout | command | layout | minus | **Delete** every guide of a spread. | - | layout | spread(node-ref)! help="The spread." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"layout.guide.move | Move | layout | command | layout | ruler | **Move** a guide to another place. | - | layout | guide(node-ref)! help="The guide.",position(number)! help="Where it stands now." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"layout.item.add | Add | layout | command | layout | plus | Put an **item** in a spread. | - | layout | spread(node-ref)! help="The spread it goes in.",name(text)! help="What to call it.",kind(text)? help="What kind of item it is.",shape(text)? help="Its shape: `rectangle`, `ellipse`, `polygon`, `line`, `path`.",rect(json)? help="Where it is, as `{x, y, width, height}`." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"layout.item.back | Back | layout | command | layout | arrows | **Send** an item to the **back**. | - | layout | item(node-ref)! help="The item." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"layout.item.backward | Backward | layout | command | layout | arrows | **Move** an item **one step** towards the back. | - | layout | item(node-ref)! help="The item." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"layout.item.fill | Fill | layout | command | layout | paint | **Fill** an item with a colour — or take the fill away. | - | layout | item(node-ref)! help="The item.",colour(text)? help="The colour of the fill. An empty colour takes the fill away; without one, the fill stays as it is." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"layout.item.forward | Forward | layout | command | layout | arrows | **Move** an item **one step** towards the front. | - | layout | item(node-ref)! help="The item." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"layout.item.front | Front | layout | command | layout | arrows | **Bring** an item to the **front**. | - | layout | item(node-ref)! help="The item." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"layout.item.group | Group | layout | command | layout | group | **Group** items. | - | layout | items(json)! help="The items to group: at least two.",name(text)? help="What to call the group." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"layout.item.hide | Hide | layout | command | layout | minus | **Hide** an item. | - | layout | item(node-ref)! help="The item." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"layout.item.layer | Layer | layout | command | layout | plus | Put an item **on a layer**. | - | layout | item(node-ref)! help="The item.",layer(node-ref)! help="The layer." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"layout.item.link | Link | layout | command | layout | link | **Set** the link of an item: a target outside the document. | - | layout | item(node-ref)! help="The item.",target(text)! help="The target: `https://example.com` or `mailto:ann@example.com`. Without it, the link is taken away." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"layout.item.list | List | layout | command | layout | list | **Read** the items of a spread, in the order they are drawn. | - | layout | spread(node-ref)! help="The spread." | mutates=false | undoable=false | journal=true | example=false"##,
    r##"layout.item.lock | Lock | layout | command | layout | lock | **Lock** an item. | - | layout | item(node-ref)! help="The item." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"layout.item.opacity | Opacity | layout | command | layout | paint | **Set** the opacity of an item. | - | layout | item(node-ref)! help="The item.",opacity(number)! help="The opacity, as a percentage: from zero to a hundred." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"layout.item.page.link | Link | layout | command | layout | link | **Set** the link of an item to a **page** of the document. | - | layout | item(node-ref)! help="The item.",page(node-ref)? help="The page it points at. Without it, the link to a page is taken away." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"layout.item.radius | Radius | layout | command | layout | rect | **Round** the corners of an item. | - | layout | item(node-ref)! help="The item.",radius(number)! help="The radius of the corners, in points." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"layout.item.rotate | Rotate | layout | command | layout | rect | **Turn** an item. | - | layout | item(node-ref)! help="The item.",degrees(number)! help="The turn, in degrees. A turn of ninety is a quarter turn, and the turn is clockwise on the page." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"layout.item.scale | Scale | layout | command | layout | rect | **Scale** an item. | - | layout | item(node-ref)! help="The item.",factor(number)! help="The factor: twice is twice as big." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"layout.item.shadow | Shadow | layout | command | layout | paint | **Give** an item a shadow. | - | layout | item(node-ref)! help="The item.",angle(number)! help="The **angle** the light comes from, in degrees.",distance(number)! help="The **distance** the shadow is moved, in points.",blur(number)? help="The **blur** the shadow is softened by, in points.",colour(text)? help="The **colour** of the shadow, as `#rrggbb`.",opacity(number)? help="The **opacity** of the shadow, from zero to one." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"layout.item.show | Show | layout | command | layout | plus | **Show** an item. | - | layout | item(node-ref)! help="The item." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"layout.item.story | Story | layout | command | layout | plus | Put a **story** in an item: the text the frame shows. | - | layout | item(node-ref)! help="The item.",story(node-ref)! help="The story." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"layout.item.stroke | Stroke | layout | command | layout | paint | **Stroke** an item with a colour — or take the stroke away. | - | layout | item(node-ref)! help="The item.",colour(text)? help="The colour of the stroke. An empty colour takes the stroke away; without one, the stroke stays as it is.",weight(number)? help="The **weight** of the stroke, in points. Without one, the weight stays as it is." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"layout.item.style | Style | layout | command | layout | plus | **Put** an item on a style — or take it off. | - | layout | item(node-ref)! help="The item.",style(node-ref)? help="The style it is on. Without one, the item is on no style." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"layout.item.ungroup | Ungroup | layout | command | layout | minus | **Ungroup** a group: the items go back where the group stood. | - | layout | group(node-ref)! help="The group." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"layout.item.unlock | Unlock | layout | command | layout | lock | **Unlock** an item. | - | layout | item(node-ref)! help="The item." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"layout.layer.add | Add | layout | command | layout | plus | Put a **layer** in a document. | - | layout | document(node-ref)! help="The document it goes in.",name(text)! help="The layer's name.",color(text)? help="Its colour, as a name or a number." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"layout.layer.delete | Delete | layout | command | layout | minus | **Delete** a layer. | - | layout | layer(node-ref)! help="The layer." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"layout.layer.hide | Hide | layout | command | layout | minus | **Hide** a layer: what is on it is not drawn. | - | layout | layer(node-ref)! help="The layer." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"layout.layer.lock | Lock | layout | command | layout | lock | **Lock** a layer: what is on it may not be edited. | - | layout | layer(node-ref)! help="The layer." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"layout.layer.merge | Merge | layout | command | layout | group | **Merge** layers: the items of them move to one, and the others are gone. | - | layout | layers(json)! help="The layers to merge: at least one.",into(node-ref)? help="The layer the items go to: the first of them when it is not named." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"layout.layer.move | Move | layout | command | layout | arrows | **Move** a layer in the order of the layers. | - | layout | layer(node-ref)! help="The layer.",to(number)! help="The place it goes: the first is the frontmost." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"layout.layer.rename | Rename | layout | command | layout | pencil | **Rename** a layer. | - | layout | layer(node-ref)! help="The layer.",name(text)! help="Its new name." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"layout.layer.show | Show | layout | command | layout | plus | **Show** a layer: what is on it is drawn again. | - | layout | layer(node-ref)! help="The layer." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"layout.layer.unlock | Unlock | layout | command | layout | lock | **Unlock** a layer: what is on it may be edited again. | - | layout | layer(node-ref)! help="The layer." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"layout.layer.unused.delete | Delete | layout | command | layout | minus | **Delete** the layers of a document that have **nothing on them**. | - | layout | document(node-ref)! help="The document." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"layout.links | Links | layout | command | layout | link | **Read** the links of a document. | - | layout | document(node-ref)! help="The document." | mutates=false | undoable=false | journal=true | example=false"##,
    r##"layout.page.add | Add | layout | command | layout | plus | Put a **page** in a spread. | - | layout | spread(node-ref)! help="The spread it goes in.",name(text)! help="What to call it.",width(number)? help="How wide it is.",height(number)? help="How high it is." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"layout.page.columns | Columns | layout | command | layout | rect | **Set** the columns of a page. | - | layout | page(node-ref)! help="The page.",count(number)! help="How many columns there are.",gutter(number)? help="The room between them. Without it, the columns have no gutter." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"layout.page.delete | Delete | layout | command | layout | minus | **Delete** a page. | - | layout | page(node-ref)! help="The page." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"layout.page.facing | Facing | layout | command | layout | page | **Read** the facing page of a page. | - | layout | page(node-ref)! help="The page." | mutates=false | undoable=false | journal=true | example=false"##,
    r##"layout.page.guides | Guides | layout | command | layout | list | The **guides** of a page: what they are and where they stand. | - | layout | page(node-ref)! help="The page." | mutates=false | undoable=false | journal=true | example=false"##,
    r##"layout.page.margin | Margin | layout | command | layout | rect | **Set** the margins of a page. | - | layout | page(node-ref)! help="The page.",top(number)? help="The room at the top of the page.",bottom(number)? help="The room at the bottom of the page.",left(number)? help="The room at the left of the page.",right(number)? help="The room at the right of the page." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"layout.page.move | Move | layout | command | layout | arrows | **Move** a page to another place among the pages of its spread. | - | layout | page(node-ref)! help="The page.",to(number)! help="The place it goes: the first of the pages is 0." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"layout.page.parent | Parent | layout | command | layout | page | **Give** a page a parent: the page it takes what it has not got of its own from. | - | layout | page(node-ref)! help="The page.",parent(node-ref)! help="The page it follows." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"layout.page.parent.clear | Clear | layout | command | layout | page | Take the **parent** of a page away. | - | layout | page(node-ref)! help="The page." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"layout.page.side | Side | layout | command | layout | page | **Say** which side of its spread a page is on. | - | layout | page(node-ref)! help="The page.",side(text)! help="The side it is on: `single`, `left` or `right`." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"layout.spread.add | Add | layout | command | layout | plus | Put a **spread** in a document. | - | layout | document(node-ref)! help="The document it goes in.",name(text)! help="What to call it." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"layout.story.add | Add | layout | command | layout | plus | Put a **story** in a document: the text, stored once. | - | layout | document(node-ref)! help="The document it goes in.",name(text)! help="What to call it.",text(text)! help="The text of the story." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"layout.story.link | Link | layout | command | layout | link | **Link** a story to another story. | - | layout | story(node-ref)! help="The story.",parent(node-ref)! help="The story it takes its content from." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"layout.story.set | Set | layout | command | layout | pencil | **Set** the text of a story. | - | layout | story(node-ref)! help="The story.",text(text)! help="Its text." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"layout.story.unlink | Unlink | layout | command | layout | minus | **Unlink** a story. | - | layout | story(node-ref)! help="The story." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"layout.story.update | Update | layout | command | layout | arrow | **Update** the links of the stories: a story takes the content of its parent again, and its own edits are replaced. | - | layout | story(node-ref)? help="The story, when it is not every link that is out of date." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"layout.style.add | Add | layout | command | layout | plus | **Add** a style to a document. | - | layout | document(node-ref)! help="The document it goes in.",name(text)! help="What to call it.",based_on(node-ref)? help="The style it is based on." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"layout.style.fill | Fill | layout | command | layout | paint | **Fill** a style with a colour — or take the fill away. | - | layout | style(node-ref)! help="The style.",colour(text)? help="The colour of the fill." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"layout.style.stroke | Stroke | layout | command | layout | paint | **Stroke** a style with a colour — or take the stroke away. | - | layout | style(node-ref)! help="The style.",colour(text)? help="The colour of the stroke.",weight(number)? help="The **weight** of the stroke, in points." | mutates=true | undoable=true | journal=true | example=false"##,
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
