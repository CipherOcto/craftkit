//! The **eleven components** — an a2ui component is a craftkit kind.
//!
//! # What each row is
//!
//! A component is a **struct whose fields are its props**, and the
//! compiler writes everything but the painter: the reader, the
//! declarations, the published schema, the typed view of a payload, the
//! kind, the catalogue entry and the two registrations
//! (`#[derive(A2uiComponent)]`).
//!
//! # The props are craftkit's; the names are a2ui's
//!
//! The **name** of a prop is a2ui's — `text`, `children`, `child` — and
//! the **type** of a prop is craftkit's — `Dynamic`, `Option<Text>`,
//! `Number`. The `#[a2ui_prop(…)]` says which **a2ui definition** the
//! prop uses, and the definition is a2ui's own name, literally:
//! `DynamicString`, `Weight`, `Child`, `ChildList`, `Action`.
//!
//! # The painter is hand-written
//!
//! The painter is ordinary craftkit code: `node.rect`,
//! `document.appearance_of(node)`, `canvas.text`, `canvas.rect`,
//! `Rect::contains`. The macro writes the **edges** and the body is the
//! programmer's — exactly as `commands!` does with `run:`.
//!
//! # A binding is resolved against the **model**
//!
//! An a2ui prop that carries a `DynamicString` may be a **literal**, a
//! **binding** (`{"@path": "/title"}`) or a **call**. The literal is
//! used, the binding is looked up in the layer's model
//! (`craftkit_a2ui::model::Model`), and a **call** is not drawn — it is
//! a round trip, and the command that carries it is the one that
//! resolves it.
//!
//! The model is the **surface's**, and the surface is in the document —
//! so the painter reads it from the document, and the painter is the
//! only place that has both.

use crate::props::Dynamic;
use craftkit_sdk::serde_json::Value as Json;
use craftkit_sdk::{BlendMode, Colour, Document, Fill, Node, Number, Point, Rect, Text};

/// The **colour** a node is drawn in.
///
/// The node's own appearance says so — and a node with no appearance is
/// drawn in the tree's own black. A **muted** node is drawn in nothing
/// at all.
fn colour_of(node: &Node, document: &Document) -> Colour {
    if node.muted {
        return Colour::NONE;
    }
    match document.appearance_of(node) {
        Some(appearance) => appearance.fill.colour(),
        None => Colour::BLACK,
    }
}

/// The **fill** a container is drawn with.
///
/// A container draws its own place, and a container with **no**
/// appearance draws nothing at all: a box that is not there is not a
/// white box.
fn container_fill_of(node: &Node, document: &Document) -> Fill {
    if node.muted {
        return Fill::Solid(Colour::NONE);
    }
    match document.appearance_of(node) {
        Some(appearance) => appearance.fill,
        None => Fill::Solid(Colour::NONE),
    }
}

/// The **fill** a box is drawn with.
///
/// A box is a box: a card, a button, a field. Its own appearance says
/// what it is filled with, and a box with no appearance is the tree's
/// own white.
fn box_fill_of(node: &Node, document: &Document) -> Fill {
    if node.muted {
        return Fill::Solid(Colour::NONE);
    }
    match document.appearance_of(node) {
        Some(appearance) => appearance.fill,
        None => Fill::Solid(Colour::WHITE),
    }
}

/// The **rectangle** a node is drawn in, if it has one.
///
/// A node with **no place** is a node that is not on a canvas, and
/// nothing is drawn for it.
fn rect_of(node: &Node) -> Option<Rect> {
    node.rect
}

craftkit_derive::components! {
    table = "craftkit_a2ui::defs",
    shapes = "craftkit_a2ui::schema",
    {
    /// A line of text.
    Text {
        kind: "a2ui.text",
        props: {
            /// The text to show.
            #[a2ui_prop(DynamicString)]
            text: Dynamic,
            /// A hint for the base text size.
            variant: Option<Text>,
            /// How much of the row this takes.
            #[a2ui_prop(Weight)]
            weight: Option<Number>,
        },
        paint: {
            let Some(props) = TextComponent::of(node) else { return };
            let Some(rect) = rect_of(node) else { return };
            let model = crate::model::model_of(node, document);
            let Some(text) = crate::props::text_of(&model, &props.text.0) else { return };
            let size = match props.variant.as_ref().map(|value| value.0.as_str()) {
                Some("caption") => 8.0,
                _ => 12.0,
            };
            canvas.text(
                Point::new(rect.x, rect.y),
                &text,
                colour_of(node, document),
                size,
            );
        },
        hit: {
            node.rect.is_some_and(|rect| rect.contains(point))
        },
    }

    /// A picture: an image the agent names.
    Image {
        kind: "a2ui.image",
        props: {
            /// Where the picture is.
            #[a2ui_prop(DynamicString)]
            url: Dynamic,
            /// A description of it, for a person.
            description: Option<Dynamic>,
            /// How the picture fills its place.
            fit: Option<Text>,
            /// A hint for the base text size.
            variant: Option<Text>,
            /// How much of the row this takes.
            #[a2ui_prop(Weight)]
            weight: Option<Number>,
        },
        paint: {
            // The tree does **not** fetch anything over a network: the
            // picture is a **file** the agent names, and the file is
            // read at paint time — so it is read again on every frame,
            // and the read is bounded.
            let Some(props) = ImageComponent::of(node) else { return };
            let Some(rect) = rect_of(node) else { return };
            let model = crate::model::model_of(node, document);
            let Some(url) = crate::props::text_of(&model, &props.url.0) else { return };
            let Some(picture) = crate::picture::picture_of(&url) else { return };
            canvas.picture(rect, &picture);
        },
        hit: {
            node.rect.is_some_and(|rect| rect.contains(point))
        },
    }

    /// An icon: a shape the agent draws.
    Icon {
        kind: "a2ui.icon",
        props: {
            /// The shape to draw.
            name: Dynamic,
            /// How much of the row this takes.
            #[a2ui_prop(Weight)]
            weight: Option<Number>,
        },
        paint: {
            let Some(props) = IconComponent::of(node) else { return };
            let Some(rect) = rect_of(node) else { return };
            let model = crate::model::model_of(node, document);
            let Some(name) = crate::props::icon_path_of(&model, &props.name.0) else { return };
            let Some(path) = crate::svg::path_of(&name) else { return };
            // The path is in the agent's own coordinates, and the icon's
            // place is where it is drawn: so the path is fitted to the
            // place, and the shape is kept.
            let Some(path) = crate::svg::fitted(&path, rect) else { return };
            canvas.path(&path, Fill::Solid(colour_of(node, document)), BlendMode::Normal, None);
        },
        hit: {
            node.rect.is_some_and(|rect| rect.contains(point))
        },
    }

    /// A row: children, side by side.
    Row {
        kind: "a2ui.row",
        props: {
            /// The children, in order.
            #[a2ui_prop(ChildList)]
            children: Json,
            /// How they are packed along the row.
            justify: Option<Text>,
            /// How they are aligned across it.
            align: Option<Text>,
            /// How much of the parent this takes.
            #[a2ui_prop(Weight)]
            weight: Option<Number>,
        },
        paint: {
            // The children are **nodes of their own**, and they paint
            // themselves: a row draws its place and nothing else.
            let Some(rect) = rect_of(node) else { return };
            canvas.rect(rect, container_fill_of(node, document), BlendMode::Normal);
        },
        hit: {
            node.rect.is_some_and(|rect| rect.contains(point))
        },
    }

    /// A column: children, one above the other.
    Column {
        kind: "a2ui.column",
        props: {
            /// The children, in order.
            #[a2ui_prop(ChildList)]
            children: Json,
            /// How they are packed down the column.
            justify: Option<Text>,
            /// How they are aligned across it.
            align: Option<Text>,
            /// How much of the parent this takes.
            #[a2ui_prop(Weight)]
            weight: Option<Number>,
        },
        paint: {
            let Some(rect) = rect_of(node) else { return };
            canvas.rect(rect, container_fill_of(node, document), BlendMode::Normal);
        },
        hit: {
            node.rect.is_some_and(|rect| rect.contains(point))
        },
    }

    /// A list: children, in a direction.
    List {
        kind: "a2ui.list",
        props: {
            /// The children, in order.
            #[a2ui_prop(ChildList)]
            children: Json,
            /// Which way they run.
            direction: Option<Text>,
            /// How they are aligned across it.
            align: Option<Text>,
            /// How much of the parent this takes.
            #[a2ui_prop(Weight)]
            weight: Option<Number>,
        },
        paint: {
            let Some(rect) = rect_of(node) else { return };
            canvas.rect(rect, container_fill_of(node, document), BlendMode::Normal);
        },
        hit: {
            node.rect.is_some_and(|rect| rect.contains(point))
        },
    }

    /// A card: a box and the child inside it.
    Card {
        kind: "a2ui.card",
        props: {
            /// The card's child.
            #[a2ui_prop(Child)]
            child: Text,
            /// How much of the parent this takes.
            #[a2ui_prop(Weight)]
            weight: Option<Number>,
        },
        paint: {
            // The child is a node of its own, under the card, and it
            // draws itself: the card draws the box.
            let Some(rect) = rect_of(node) else { return };
            canvas.rect(rect, box_fill_of(node, document), BlendMode::Normal);
        },
        hit: {
            node.rect.is_some_and(|rect| rect.contains(point))
        },
    }

    /// A divider: a line.
    Divider {
        kind: "a2ui.divider",
        props: {
            /// Which way the line runs.
            axis: Option<Text>,
            /// How much of the parent this takes.
            #[a2ui_prop(Weight)]
            weight: Option<Number>,
        },
        paint: {
            let Some(rect) = rect_of(node) else { return };
            let horizontal = !matches!(props_axis(node).as_deref(), Some("vertical"));
            let fill = if horizontal {
                Rect::new(rect.x, rect.y, rect.w, 1.0)
            } else {
                Rect::new(rect.x, rect.y, 1.0, rect.h)
            };
            canvas.rect(fill, Fill::Solid(colour_of(node, document)), BlendMode::Normal);
        },
        hit: {
            node.rect.is_some_and(|rect| rect.contains(point))
        },
    }

    /// A button: something to press.
    Button {
        kind: "a2ui.button",
        props: {
            /// What the button says — the id of the child that says it.
            #[a2ui_prop(Child)]
            child: Text,
            /// A hint for the base text size.
            variant: Option<Text>,
            /// What the button does.
            #[a2ui_prop(Action)]
            action: Option<Json>,
            /// How much of the row this takes.
            #[a2ui_prop(Weight)]
            weight: Option<Number>,
        },
        paint: {
            // The button draws the box, and the child — a text, say —
            // is a node of its own under it, and it draws itself.
            let Some(rect) = rect_of(node) else { return };
            canvas.rect(rect, box_fill_of(node, document), BlendMode::Normal);
        },
        hit: {
            node.rect.is_some_and(|rect| rect.contains(point))
        },
    }

    /// A check box: something to tick.
    CheckBox {
        kind: "a2ui.checkbox",
        props: {
            /// What the box says.
            #[a2ui_prop(DynamicString)]
            label: Dynamic,
            /// Whether it is ticked.
            #[a2ui_prop(DynamicBoolean)]
            value: Option<Dynamic>,
            /// How much of the row this takes.
            #[a2ui_prop(Weight)]
            weight: Option<Number>,
        },
        paint: {
            let Some(props) = CheckBoxComponent::of(node) else { return };
            let Some(rect) = rect_of(node) else { return };
            let model = crate::model::model_of(node, document);
            canvas.rect(rect, box_fill_of(node, document), BlendMode::Normal);
            let Some(label) = crate::props::text_of(&model, &props.label.0) else { return };
            let colour = colour_of(node, document);
            canvas.text(Point::new(rect.x + 14.0, rect.y), &label, colour, 12.0);
            // The **tick**: the box is ticked when the value says so.
            let ticked = match &props.value {
                Some(value) => crate::props::bool_of(&model, &value.0).unwrap_or(false),
                None => false,
            };
            if ticked {
                canvas.rect(
                    Rect::new(rect.x + 2.0, rect.y + 2.0, 8.0, 8.0),
                    Fill::Solid(colour),
                    BlendMode::Normal,
                );
            }
        },
        hit: {
            node.rect.is_some_and(|rect| rect.contains(point))
        },
    }

    /// A text field: something to type into.
    TextField {
        kind: "a2ui.textfield",
        props: {
            /// What the field says.
            #[a2ui_prop(DynamicString)]
            label: Dynamic,
            /// What is in the field.
            #[a2ui_prop(DynamicString)]
            value: Option<Dynamic>,
            /// A hint for what to type.
            placeholder: Option<Dynamic>,
            /// A hint for the base text size.
            variant: Option<Text>,
            /// How much of the row this takes.
            #[a2ui_prop(Weight)]
            weight: Option<Number>,
        },
        paint: {
            let Some(props) = TextFieldComponent::of(node) else { return };
            let Some(rect) = rect_of(node) else { return };
            let model = crate::model::model_of(node, document);
            canvas.rect(rect, box_fill_of(node, document), BlendMode::Normal);
            let colour = colour_of(node, document);
            if let Some(label) = crate::props::text_of(&model, &props.label.0) {
                canvas.text(Point::new(rect.x + 2.0, rect.y), &label, colour, 8.0);
            }
            // The **value**: what is in the field, and the placeholder
            // when there is none.
            let value = props
                .value
                .as_ref()
                .and_then(|value| crate::props::text_of(&model, &value.0));
            let shown = match value {
                Some(value) => value,
                None => match &props.placeholder {
                    Some(placeholder) => match crate::props::text_of(&model, &placeholder.0) {
                        Some(placeholder) => placeholder,
                        None => return,
                    },
                    None => return,
                },
            };
            canvas.text(Point::new(rect.x + 2.0, rect.y + 10.0), &shown, colour, 12.0);
        },
        hit: {
            node.rect.is_some_and(|rect| rect.contains(point))
        },
    }
    }
}

/// The **axis** a divider runs along, if it says one.
fn props_axis(node: &Node) -> Option<String> {
    let component = DividerComponent::of(node)?;
    component.axis.map(|value| value.0)
}
