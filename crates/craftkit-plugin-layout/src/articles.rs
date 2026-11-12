//! The **articles** of a layout document.
//!
//! An article is a **named list of items** in a reading order: the
//! order they are read in, and not the order they stand in.
//!
//! # The rules
//!
//! These are the tree's own rules.
//!
//! - An article has a **name**, and no two articles have the same one.
//!   An article that is added without a name is called `Article N`,
//!   where N is the first number that is free.
//! - The **items** of an article are the items of the document, in the
//!   order they were added. An item that is added twice is not in the
//!   article twice, and an item that is not there is not in it at all.
//! - An article may be **renamed**, its items may be **taken out**,
//!   and it may be **deleted**. An article that is not there is
//!   refused by name.
//! - The **order** of the items may be set: the items that are named
//!   come first, in the order they are named, and the rest follow.
//! - An article says whether it is **exported**, and it is exported
//!   until it is told otherwise.
//!
//! # What it does not do
//!
//! - **The tree's items are the selection** when no items are named,
//!   and the tree's articles are what the EPUB and the HTML are
//!   exported in. There is no selection here and nothing is exported.

use craftkit_derive::commands;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{Bool, EngineError, Node, NodeRef, Outcome, Session, Text};

/// The most an article's **name** may have.
const MAX_NAME: usize = 256;

/// The most **items** an article may name, and the most a command may
/// be given.
const MAX_MEMBERS: usize = 10_000;

/// The **articles** of a document, in the order they were made.
fn articles_of(document: &Node) -> Vec<Node> {
    document
        .children
        .iter()
        .filter(|child| child.kind == "layout.article")
        .map(|child| (**child).clone())
        .collect()
}

/// The **items** of an article, in the order they are in.
fn items_of(article: &Node) -> Vec<u64> {
    article
        .data
        .get("items")
        .and_then(|items| items.as_array())
        .map(|items| items.iter().filter_map(|item| item.as_u64()).collect())
        .unwrap_or_default()
}

/// Put the **items** of an article back, in the order they are in.
fn set_items(article: &mut Node, items: &[u64]) {
    article.data["items"] = json!(items);
}

/// The **article** a command is about.
fn the_article<'a>(session: &'a Session, command: &str, article: u64) -> Result<&'a Node, EngineError> {
    let Some(node) = session.document().node(article) else {
        return Err(EngineError::Failed {
            id: command.to_string(),
            message: format!("there is no article `{article}`"),
        });
    };
    if node.kind != "layout.article" {
        return Err(EngineError::Failed {
            id: command.to_string(),
            message: format!("the node `{article}` is a `{}` and not an article", node.kind),
        });
    }
    Ok(node)
}

/// The **items** a command is given: a list of ids, and every one of
/// them is a whole number.
fn ids_of(command: &str, value: Option<Json>) -> Result<Vec<u64>, EngineError> {
    let Some(value) = value else { return Ok(Vec::new()) };
    let Some(items) = value.as_array() else {
        return Err(EngineError::Failed {
            id: command.to_string(),
            message: "the items are a list of ids".to_string(),
        });
    };
    if items.len() > MAX_MEMBERS {
        return Err(EngineError::Failed {
            id: command.to_string(),
            message: format!("a list of items holds at most {MAX_MEMBERS} ids"),
        });
    }
    let mut ids = Vec::with_capacity(items.len());
    for item in items {
        let Some(id) = item.as_u64() else {
            return Err(EngineError::Failed {
                id: command.to_string(),
                message: "the items are a list of ids".to_string(),
            });
        };
        ids.push(id);
    }
    Ok(ids)
}

/// Whether an id is an **item** of the document: the tree's article
/// holds objects, and an object that is not there is not held.
fn is_item(session: &Session, id: u64) -> bool {
    session.document().node(id).is_some_and(|node| node.kind.starts_with("layout.item."))
}

/// The **articles** of a document, as the answer holds them.
fn articles_json(articles: &[Node]) -> Vec<Json> {
    articles
        .iter()
        .map(|article| {
            json!({
                "id": article.id,
                "name": article.data.get("name").cloned().unwrap_or(Json::Null),
                "items": items_of(article),
                "export": article.data.get("export").and_then(|export| export.as_bool()).unwrap_or(true),
            })
        })
        .collect()
}

commands! {
    /// The **articles** of a document: their names, their items and whether
    /// they are exported.
    layout.document.articles {
        area: "layout",
        layer: "layout",
        icon: "list",
        params: {
            /// The document.

            document: NodeRef,
        },
        run: {

            let command = "layout.document.articles";
            let document = self.document.0;
            let Some(node) = session.document().node(document) else {
                return Err(EngineError::Failed {
                    id: command.to_string(),
                    message: format!("there is no document `{document}`"),
                });
            };
            let articles = articles_json(&articles_of(node));
            Ok(Outcome::new(format!("the document `{document}` holds {} article(s)", articles.len())).with_data(
                json!({
                    "document": document,
                    "articles": articles,
                    "count": articles.len(),
                }),
            ))

        }
    }
    /// **New** article in a document.
    ///
    /// The name is `Article N` when it is not given, and a name that is
    /// taken is refused.
    layout.article.new {
        area: "layout",
        layer: "layout",
        icon: "plus",
        mutates,
        undoable,
        params: {
            /// The document.

            document: NodeRef,
            /// The name of the article.

            name: Option<Text>,
            /// The items it holds, in the order they are in.

            items: Option<Json>,
        },
        run: {

            let command = "layout.article.new";
            let document = self.document.0;
            let Some(node) = session.document().node(document).cloned() else {
                return Err(EngineError::Failed {
                    id: command.to_string(),
                    message: format!("there is no document `{document}`"),
                });
            };
            let articles = articles_of(&node);
            // The **name**: the one that is given, and the first free
            // `Article N` when it is not.
            let name = match self.name.as_ref().map(|name| name.0.trim().to_string()) {
                Some(name) if name.is_empty() => {
                    return Err(EngineError::Failed {
                        id: command.to_string(),
                        message: "an article has a name".to_string(),
                    });
                }
                Some(name) => {
                    if name.chars().count() > MAX_NAME {
                        return Err(EngineError::Failed {
                            id: command.to_string(),
                            message: format!("the name of an article is not longer than {MAX_NAME} characters"),
                        });
                    }
                    if articles.iter().any(|article| {
                        article.data.get("name").and_then(|name| name.as_str()).unwrap_or_default() == name
                    }) {
                        return Err(EngineError::Failed {
                            id: command.to_string(),
                            message: format!("an article named `{name}` exists"),
                        });
                    }
                    name
                }
                None => (1..=articles.len() + 1)
                    .map(|number| format!("Article {number}"))
                    .find(|name| {
                        !articles.iter().any(|article| {
                            article.data.get("name").and_then(|name| name.as_str()).unwrap_or_default() == name
                        })
                    })
                    .ok_or_else(|| EngineError::Failed {
                        id: command.to_string(),
                        message: "no free article name".to_string(),
                    })?,
            };
            // The **items**: the ones that are there.
            let given = ids_of(command, self.items.clone())?;
            let items: Vec<u64> = given.into_iter().filter(|id| is_item(session, *id)).collect();
            let id = session.document_mut()?.add_node_with_under(
                Some(document),
                name.clone(),
                "layout.article",
                None,
                json!({ "name": name, "items": items, "export": true }),
            );
            Ok(Outcome::changed(format!("added the article `{name}`")).with_data(json!({
                "id": id,
                "document": document,
                "name": name,
                "items": items,
            })))

        }
    }
    /// **Add** items to an article, in the order they are given.
    ///
    /// An item that is already in the article is not in it twice, and an
    /// item that is not there is not in it at all.
    layout.article.add {
        area: "layout",
        layer: "layout",
        icon: "plus",
        mutates,
        undoable,
        params: {
            /// The article.

            article: NodeRef,
            /// The items.

            items: Option<Json>,
        },
        run: {

            let command = "layout.article.add";
            let article = self.article.0;
            the_article(session, command, article)?;
            let given = ids_of(command, self.items.clone())?;
            let mut items = items_of(the_article(session, command, article)?);
            let mut added = 0usize;
            for id in given {
                if !is_item(session, id) || items.contains(&id) {
                    continue;
                }
                items.push(id);
                added += 1;
            }
            if let Some(node) = session.document_mut()?.node_mut(article) {
                set_items(node, &items);
            }
            Ok(Outcome::changed(format!("added {added} item(s) to the article `{article}`")).with_data(json!({
                "article": article,
                "added": added,
                "items": items,
            })))

        }
    }
    /// **Take** items out of an article.
    layout.article.remove {
        area: "layout",
        layer: "layout",
        icon: "minus",
        mutates,
        undoable,
        params: {
            /// The article.

            article: NodeRef,
            /// The items.

            items: Option<Json>,
        },
        run: {

            let command = "layout.article.remove";
            let article = self.article.0;
            the_article(session, command, article)?;
            let given = ids_of(command, self.items.clone())?;
            let mut items = items_of(the_article(session, command, article)?);
            let before = items.len();
            items.retain(|id| !given.contains(id));
            let removed = before - items.len();
            if let Some(node) = session.document_mut()?.node_mut(article) {
                set_items(node, &items);
            }
            Ok(Outcome::changed(format!("took {removed} item(s) out of the article `{article}`")).with_data(
                json!({
                    "article": article,
                    "removed": removed,
                    "items": items,
                }),
            ))

        }
    }
    /// The **options** of an article: its name, whether it is exported and
    /// the order of its items.
    layout.article.options {
        area: "layout",
        layer: "layout",
        icon: "pencil",
        mutates,
        undoable,
        params: {
            /// The article.

            article: NodeRef,
            /// Its new name.

            to: Option<Text>,
            /// Whether it is exported.

            export: Option<Bool>,
            /// The order of its items: these come first, and the rest follow.

            order: Option<Json>,
        },
        run: {

            let command = "layout.article.options";
            let article = self.article.0;
            the_article(session, command, article)?;
            let mut items = items_of(the_article(session, command, article)?);
            // The **order**: the items that are named come first, and the
            // rest follow in the order they were in.
            if let Some(order) = self.order.clone() {
                let named = ids_of(command, Some(order))?;
                let mut ordered: Vec<u64> = named.into_iter().filter(|id| items.contains(id)).collect();
                let rest: Vec<u64> = items.iter().filter(|id| !ordered.contains(id)).copied().collect();
                ordered.extend(rest);
                items = ordered;
            }
            let name = self.to.as_ref().map(|name| name.0.trim().to_string());
            if let Some(name) = name.as_ref() {
                if name.is_empty() {
                    return Err(EngineError::Failed {
                        id: command.to_string(),
                        message: "an article has a name".to_string(),
                    });
                }
                if name.chars().count() > MAX_NAME {
                    return Err(EngineError::Failed {
                        id: command.to_string(),
                        message: format!("the name of an article is not longer than {MAX_NAME} characters"),
                    });
                }
            }
            let export = self.export.map(|flag| flag.0);
            if let Some(node) = session.document_mut()?.node_mut(article) {
                set_items(node, &items);
                if let Some(name) = name.clone() {
                    node.name = name.clone();
                    node.data["name"] = json!(name);
                }
                if let Some(export) = export {
                    node.data["export"] = json!(export);
                }
            }
            Ok(Outcome::changed(format!("the options of the article `{article}` are set")).with_data(json!({
                "article": article,
                "items": items,
                "export": export,
                "name": name,
            })))

        }
    }
    /// **Delete** an article.
    layout.article.delete {
        area: "layout",
        layer: "layout",
        icon: "minus",
        mutates,
        undoable,
        params: {
            /// The article.

            article: NodeRef,
        },
        run: {

            let command = "layout.article.delete";
            let article = self.article.0;
            the_article(session, command, article)?;
            session.document_mut()?.remove_node(article);
            Ok(Outcome::changed(format!("the article `{article}` is gone"))
                .with_data(json!({ "article": article })))

        }
    }
}
