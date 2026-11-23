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
//! the **type** of a prop is craftkit's — `Text`, `Option<Text>`,
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
//! (`craftkit_a2ui::model::Model`), and a **call** is not drawn — it is a
//! round trip, and the command that carries it is the one that resolves
//! it.

use craftkit_a2ui::bindings;
use craftkit_sdk::serde_json::Value as Json;
use craftkit_sdk::{BlendMode, Bool, Colour, Document, Fill, Node, Number, Point, Rect, Text};

/// The **text** of a prop, resolved against the model.
///
/// A literal is used, a binding is looked up, and a call is **refused** —
/// a call has no text until the round trip is made, and a painter is not
/// where a round trip is made.
pub fn text_of(value: &Json) -> Option<String> {
    crate::model::resolve(|model| bindings::resolve_text(model, value).ok())
}

/// The **boolean** of a prop, resolved against the model.
pub fn bool_of(value: &Json) -> Option<bool> {
    crate::model::resolve(|model| bindings::resolve_bool(model, value).ok())
}

/// The **colour** a node is drawn in.
///
/// The node's own appearance says so, and a node with no appearance is
/// drawn in the tree's own black.
fn colour_of(node: &Node, document: &Document) -> Colour {
    let _ = document.appearance_of(node);
    if node.muted { Colour::NONE } else { Colour::BLACK }
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
            text: Text,
            /// A hint for the base text size.
            variant: Option<Text>,
            /// How much of the row this takes.
            #[a2ui_prop(Weight)]
            weight: Option<Number>,
        },
        paint: {
            let Some(props) = TextComponent::of(node) else { return };
            let Some(rect) = rect_of(node) else { return };
            let Some(text) = text_of(&Json::String(props.text.0.clone())) else { return };
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
            url: Text,
            /// What the picture is, for a person.
            description: Option<Text>,
            /// How the picture fills its place.
            fit: Option<Text>,
            /// A hint for the base text size.
            variant: Option<Text>,
            /// How much of the row this takes.
            #[a2ui_prop(Weight)]
            weight: Option<Number>,
        },
        paint: {
            // The tree does **not** fetch anything over a network, and it
            // does not read a file the agent names: a picture is drawn
            // as its place, and the place is what the agent asked for.
            let Some(rect) = rect_of(node) else { return };
            canvas.rect(rect, Fill::Solid(Colour::NONE), BlendMode::Normal);
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
            name: Text,
            /// How much of the row this takes.
            #[a2ui_prop(Weight)]
            weight: Option<Number>,
        },
        paint: {
            let Some(props) = IconComponent::of(node) else { return };
            // The **path** is the icon: a2ui's own names are a vocabulary
            // craftkit does not have, so the name is read and the shape
            // is drawn from it when it is a path.
            let Some(path) = crate::svg::path_of(&props.name.0) else { return };
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
            canvas.rect(rect, Fill::Solid(Colour::NONE), BlendMode::Normal);
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
            canvas.rect(rect, Fill::Solid(Colour::NONE), BlendMode::Normal);
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
            canvas.rect(rect, Fill::Solid(Colour::NONE), BlendMode::Normal);
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
            let Some(rect) = rect_of(node) else { return };
            canvas.rect(rect, Fill::Solid(Colour::WHITE), BlendMode::Normal);
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
            canvas.rect(fill, Fill::Solid(Colour::BLACK), BlendMode::Normal);
        },
        hit: {
            node.rect.is_some_and(|rect| rect.contains(point))
        },
    }

    /// A button: something to press.
    Button {
        kind: "a2ui.button",
        props: {
            /// What the button says.
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
            let Some(props) = ButtonComponent::of(node) else { return };
            let Some(rect) = rect_of(node) else { return };
            canvas.rect(rect, Fill::Solid(Colour::WHITE), BlendMode::Normal);
            let Some(text) = text_of(&Json::String(props.child.0.clone())) else { return };
            canvas.text(Point::new(rect.x, rect.y), &text, colour_of(node, document), 12.0);
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
            label: Text,
            /// Whether it is ticked.
            #[a2ui_prop(DynamicBoolean)]
            value: Option<Bool>,
            /// How much of the row this takes.
            #[a2ui_prop(Weight)]
            weight: Option<Number>,
        },
        paint: {
            let Some(props) = CheckBoxComponent::of(node) else { return };
            let Some(rect) = rect_of(node) else { return };
            canvas.rect(rect, Fill::Solid(Colour::WHITE), BlendMode::Normal);
            let Some(text) = text_of(&Json::String(props.label.0.clone())) else { return };
            canvas.text(Point::new(rect.x, rect.y), &text, colour_of(node, document), 12.0);
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
            label: Text,
            /// What is in the field.
            #[a2ui_prop(DynamicString)]
            value: Option<Text>,
            /// A hint for what to type.
            placeholder: Option<Text>,
            /// A hint for the base text size.
            variant: Option<Text>,
            /// How much of the row this takes.
            #[a2ui_prop(Weight)]
            weight: Option<Number>,
        },
        paint: {
            let Some(props) = TextFieldComponent::of(node) else { return };
            let Some(rect) = rect_of(node) else { return };
            canvas.rect(rect, Fill::Solid(Colour::WHITE), BlendMode::Normal);
            let Some(text) = text_of(&Json::String(props.label.0.clone())) else { return };
            canvas.text(Point::new(rect.x, rect.y), &text, colour_of(node, document), 12.0);
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
