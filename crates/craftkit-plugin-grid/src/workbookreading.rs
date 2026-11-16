//! **Reading** a workbook.
//!
//! A workbook may be read whole: the answer is everything the workbook
//! holds and everything that is said about it.
//!
//! The facts of a workbook are its **name**, the **names** it defines,
//! the **styles** it carries, the **lists** of its own words, and the
//! **calendar** its serials are counted in.
//!
//! One part of the answer is **derived**: the **sheets** of the
//! workbook are its children, and the **used range** of each sheet is
//! the stretch of it that holds something.

use craftkit_derive::commands;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{EngineError, NodeRef, Outcome, Text};

/// The **properties** of a workbook: what is said about it.
///
/// A property that is not said is nothing, and the answer says so.
pub(crate) fn properties_of(node: &craftkit_sdk::Node) -> Json {
    let properties = node.data.get("properties").cloned().unwrap_or(Json::Null);
    if properties.is_object() {
        return properties;
    }
    json!({
        "title": Json::Null,
        "subject": Json::Null,
        "author": Json::Null,
        "keywords": Json::Null,
        "description": Json::Null,
        "last_modified_by": Json::Null,
        "company": Json::Null,
    })
}

/// The **theme** of a workbook: the colours and the fonts it names.
///
/// A theme that is not said is nothing, and the answer says so.
pub(crate) fn theme_of(node: &craftkit_sdk::Node) -> Json {
    let theme = node.data.get("theme").cloned().unwrap_or(Json::Null);
    if theme.is_object() {
        return theme;
    }
    json!({
        "name": Json::Null,
        "colours": Json::Null,
        "major_font": Json::Null,
        "minor_font": Json::Null,
    })
}

commands! {
    /// Read a **workbook**.
    grid.workbook.get {
        area: "grid",
        layer: "grid",
        icon: "list",
        params: {
            /// The workbook to read.
            workbook: NodeRef,
        },
        run: {
            let id = "grid.workbook.get".to_string();
            let workbook = self.workbook.0;
            let Some(node) = session.document().node(workbook) else {
                return Err(EngineError::Failed { id, message: format!("there is no workbook `{workbook}`") });
            };
            if node.kind != "grid.workbook" {
                return Err(EngineError::Failed {
                    id,
                    message: format!("the node `{workbook}` is a `{}` and not a workbook", node.kind),
                });
            }
            // The **sheets** of the workbook, and the used range of each.
            let mut sheets: Vec<Json> = Vec::new();
            for sheet in node.children.iter().filter(|child| child.kind == "grid.sheet") {
                let cells = crate::cells_of(sheet);
                let used = crate::reading::used_range(&cells).map(|area| area.to_text());
                sheets.push(json!({
                    "id": sheet.id,
                    "name": sheet.name,
                    "used_range": used,
                    "cells": cells.len(),
                }));
            }
            let names = node.data.get("names").cloned().unwrap_or(Json::Null);
            let styles = node.data.get("styles").cloned().unwrap_or(Json::Null);
            let lists = node.data.get("custom_lists").cloned().unwrap_or(Json::Null);
            let system = node
                .data
                .get("date_system")
                .and_then(|system| system.as_str())
                .map(|system| system.to_string())
                .unwrap_or_else(|| crate::dates::System::default().name().to_string());
            Ok(Outcome::new(format!("the workbook `{}`", node.name)).with_data(json!({
                "id": workbook,
                "name": node.name,
                "sheets": sheets,
                "names": names,
                "styles": styles,
                "custom_lists": lists,
                "date_system": system,
                "properties": properties_of(node),
                "theme": theme_of(node),
            })))

        }
    }

    /// **Set** the **properties** of a workbook: what is said about it.
    ///
    /// A property that is not given keeps what it was, and a property
    /// given as an empty text is taken away.
    grid.workbook.properties.set {
        area: "grid",
        layer: "grid",
        icon: "pencil",
        mutates,
        undoable,
        params: {
            /// The workbook whose properties are set.
            workbook: NodeRef,
            /// The **title** of the workbook.
            title: Option<Text>,
            /// The **subject** of the workbook.
            subject: Option<Text>,
            /// The **author** of the workbook.
            author: Option<Text>,
            /// The **keywords** of the workbook.
            keywords: Option<Text>,
            /// The **description** of the workbook.
            description: Option<Text>,
            /// The **company** the workbook belongs to.
            company: Option<Text>,
        },
        run: {
            let id = "grid.workbook.properties.set".to_string();
            let workbook = self.workbook.0;
            let Some(node) = session.document().node(workbook) else {
                return Err(EngineError::Failed { id, message: format!("there is no workbook `{workbook}`") });
            };
            if node.kind != "grid.workbook" {
                return Err(EngineError::Failed {
                    id,
                    message: format!("the node `{workbook}` is a `{}` and not a workbook", node.kind),
                });
            }
            // The **properties** that are given: the others keep what
            // they were.
            let mut properties = properties_of(node);
            let mut set = |key: &str, value: Option<&Text>| {
                let Some(value) = value else { return };
                let text = value.0.trim().to_string();
                let entry = if text.is_empty() { Json::Null } else { json!(text) };
                if let Some(object) = properties.as_object_mut() {
                    object.insert(key.to_string(), entry);
                }
            };
            set("title", self.title.as_ref());
            set("subject", self.subject.as_ref());
            set("author", self.author.as_ref());
            set("keywords", self.keywords.as_ref());
            set("description", self.description.as_ref());
            set("company", self.company.as_ref());
            let Some(node) = session.document_mut()?.node_mut(workbook) else {
                return Err(EngineError::Failed { id, message: format!("there is no workbook `{workbook}`") });
            };
            node.data["properties"] = properties.clone();
            Ok(Outcome::changed(format!("the properties of the workbook `{}` are set", node.name))
                .with_data(json!({
                    "workbook": workbook,
                    "properties": properties,
                })))

        }
    }

    /// The **theme** of a workbook: the colours and the fonts it names.
    ///
    /// A theme that is not said is nothing, and the answer says so.
    grid.workbook.theme {
        area: "grid",
        layer: "grid",
        icon: "palette",
        params: {
            /// The workbook whose theme is read.
            workbook: NodeRef,
        },
        run: {
            let id = "grid.workbook.theme".to_string();
            let workbook = self.workbook.0;
            let Some(node) = session.document().node(workbook) else {
                return Err(EngineError::Failed { id, message: format!("there is no workbook `{workbook}`") });
            };
            if node.kind != "grid.workbook" {
                return Err(EngineError::Failed {
                    id,
                    message: format!("the node `{workbook}` is a `{}` and not a workbook", node.kind),
                });
            }
            let theme = theme_of(node);
            Ok(Outcome::new("the theme of the workbook".to_string())
                .with_data(json!({
                    "workbook": workbook,
                    "theme": theme,
                })))

        }
    }
}
