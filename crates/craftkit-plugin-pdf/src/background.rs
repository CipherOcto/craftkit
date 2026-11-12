//! The **background** of a page: a colour behind the content of it.
//!
//! The tree's own: a background is page content that is marked as a
//! **pagination artefact**, and it is drawn behind everything else.
//!
//! # The rules
//!
//! - The background has a **colour**, and the tree's default is white.
//! - The **opacity** is between zero and one, and an opacity outside
//!   that is **refused**.
//! - The **rectangle** covers the whole page: from its lower left
//!   corner, as wide and as high as the page.
//! - The background is drawn **behind** the content of the page.
//!
//! # What it does not settle
//!
//! - The **picture** background of the tree is not here: that is an
//!   image, and this is a colour.

use craftkit_derive::commands;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{EngineError, NodeRef, Number, Outcome, Text};

use crate::watermark::{box_of, number};

/// The name of the command, in the messages.
const COMMAND: &str = "pdf.page.background";

/// The **content** of the background: the tree's own operators.
fn content_of(colour: [f64; 3], opacity: f64, width: f64, height: f64) -> String {
    format!(
        "q\n/Artifact <</Type /Pagination /Subtype /Background /PCMark /Background>> BDC\n/PCGS{} gs\n{} {} {} rg\n0 0 {} {} re f\nEMC\nQ\n",
        (opacity * 100.0).round() as i64,
        number(colour[0]),
        number(colour[1]),
        number(colour[2]),
        number(width),
        number(height)
    )
}

/// The **streams** of the page's content: the nodes of them, in the
/// order the content is in.
fn content_streams(document: &craftkit_sdk::Document, page: &craftkit_sdk::Node) -> Vec<u64> {
    let entries = crate::entries_of(&page.data);
    let Some((_, value)) = entries.into_iter().find(|(key, _)| key == "Contents") else {
        return Vec::new();
    };
    let mut out: Vec<u64> = Vec::new();
    let mut push = |value: Json| {
        let Some(number) = value
            .get("ref")
            .and_then(|reference| reference.get(0))
            .and_then(Json::as_f64)
            .filter(|number| number.is_finite() && number.fract() == 0.0 && *number >= 0.0)
            .map(|number| number as u64)
        else {
            return;
        };
        if let Some(node) = document
            .walk()
            .find(|node| crate::the_number_of(node) == Some(number) && node.kind == "pdf.stream")
        {
            out.push(node.id);
        }
    };
    match value {
        Json::Array(list) => {
            for item in list {
                push(item.clone());
            }
        }
        other => push(other.clone()),
    }
    out
}

commands! {
    /// **Draw** a background on a page: a colour behind the content of it.
    ///
    /// The tree's own: the colour, the opacity, and the rectangle that
    /// covers the page.
    pdf.page.background {
        area: "pdf",
        layer: "pdf",
        icon: "pencil",
        mutates,
        undoable,
        params: {
            /// The page the background is on.

            page: NodeRef,
            /// The colour of the background: white is the tree's.

            colour: Option<Text>,
            /// The opacity of the background, between zero and one.

            opacity: Option<Number>,
        },
        run: {

            let colour = match &self.colour {
                Some(colour) => {
                    let parsed = craftkit_sdk::document::Colour::from_hex(colour.0.trim())
                        .map_err(|message| EngineError::Failed { id: COMMAND.to_string(), message })?;
                    let [r, g, b, _] = parsed.0;
                    [f64::from(r), f64::from(g), f64::from(b)]
                }
                None => [1.0, 1.0, 1.0],
            };
            let opacity = self.opacity.map(|opacity| opacity.0).unwrap_or(1.0);
            if !opacity.is_finite() || !(0.0..=1.0).contains(&opacity) {
                return Err(EngineError::Failed {
                    id: COMMAND.to_string(),
                    message: "the opacity of the background is between zero and one".to_string(),
                });
            }
            // The **page**.
            let Some(node) = session.document().node(self.page.0) else {
                return Err(EngineError::Failed {
                    id: COMMAND.to_string(),
                    message: format!("there is no node `{}`", self.page.0),
                });
            };
            let box_ = box_of(node);
            let (width, height) = (box_[2] - box_[0], box_[3] - box_[1]);
            let content = content_of(colour, opacity, width, height);
            // The page's **content**: the background is drawn behind it.
            let streams = content_streams(session.document(), node);
            let Some(stream) = streams.first() else {
                return Err(EngineError::Failed {
                    id: COMMAND.to_string(),
                    message: "the page has no content to draw the background on".to_string(),
                });
            };
            let stream_node = session.document().node(*stream).ok_or_else(|| EngineError::Failed {
                id: COMMAND.to_string(),
                message: format!("there is no node `{stream}`"),
            })?;
            let Some(path) = stream_node.stream.clone() else {
                return Err(EngineError::Failed {
                    id: COMMAND.to_string(),
                    message: format!("the stream `{stream}` has no file"),
                });
            };
            let limits = crate::filters::Limits::default();
            let bytes = craftkit_sdk::files::read_bounded(std::path::Path::new(&path), limits.max_input as u64)
                .map_err(|message| EngineError::Failed {
                id: COMMAND.to_string(),
                message: format!("the page's content cannot be read: {message}"),
            })?;
            let mut written = Vec::with_capacity(bytes.len() + content.len());
            written.extend_from_slice(content.as_bytes());
            written.extend_from_slice(&bytes);
            std::fs::write(&path, &written).map_err(|error| EngineError::Failed {
                id: COMMAND.to_string(),
                message: format!("the page's content cannot be written: {error}"),
            })?;
            // The page's **resources**: the graphics state of the opacity.
            if let Some(page_node) = session.document_mut()?.node_mut(self.page.0) {
                let mut entries = crate::entries_of(&page_node.data);
                let mut resources = entries
                    .iter()
                    .find(|(key, _)| key == "Resources")
                    .map(|(_, value)| crate::entries_of(value))
                    .unwrap_or_default();
                let mut states = resources
                    .iter()
                    .find(|(key, _)| key == "ExtGState")
                    .map(|(_, value)| crate::entries_of(value))
                    .unwrap_or_default();
                let name = format!("PCGS{}", (opacity * 100.0).round() as i64);
                states.retain(|(key, _)| *key != name);
                states.push((
                    name,
                    json!({ "entries": [
                        ["Type", "ExtGState"],
                        ["CA", opacity],
                        ["ca", opacity],
                    ] }),
                ));
                resources.retain(|(key, _)| key != "ExtGState");
                resources.push((
                    "ExtGState".to_string(),
                    json!({ "entries": states.iter().map(|(key, value)| json!([key, value])).collect::<Vec<Json>>() }),
                ));
                entries.retain(|(key, _)| key != "Resources");
                entries.push((
                    "Resources".to_string(),
                    json!({ "entries": resources.iter().map(|(key, value)| json!([key, value])).collect::<Vec<Json>>() }),
                ));
                page_node.data = json!({
                    "object": page_node.data.get("object").cloned().unwrap_or(Json::Null),
                    "entries": entries.iter().map(|(key, value)| json!([key, value])).collect::<Vec<Json>>(),
                });
            }
            Ok(Outcome::changed("the background is drawn on the page".to_string()).with_data(json!({
                "page": self.page.0,
                "colour": colour,
                "opacity": opacity,
                "bytes": content.len(),
            })))

        }
    }
}
