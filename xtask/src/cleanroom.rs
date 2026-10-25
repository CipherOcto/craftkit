//! The clean-room gate: the tree may not carry another product's material.
//!
//! The rule is in `docs/clean-room.md`, and the table of products is read from
//! there. The rule and the list of names live together, so a product named in
//! the rule is a product the gate checks, and a rule that loses its table is a
//! gate that checks nothing — which is reported rather than passed.
//!
//! What the gate can see and what it cannot is written in the rule itself.
//! §168.

use std::path::Path;

/// One product the tree may not carry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Prohibited {
    /// The product's name, as the rule writes it.
    pub name: String,
    /// What kind of product it is: `commercial` or `copyleft`.
    pub kind: String,
}

/// The rule's table of products.
///
/// The table is found by its header — `| Product | Kind | … |` — so the rest of
/// the document may hold any other table it likes. A row with an empty cell is
/// a broken rule, and a broken rule is reported: a row that says nothing about
/// a product would silently exempt it.
pub fn prohibited(text: &str) -> Result<Vec<Prohibited>, String> {
    let mut out = Vec::new();
    let mut in_table = false;
    for (number, line) in text.lines().enumerate() {
        let line = line.trim();
        if !line.starts_with('|') {
            in_table = false;
            continue;
        }
        let cells: Vec<&str> = line.split('|').skip(1).map(str::trim).collect();
        let Some(name) = cells.first() else { continue };
        if *name == "Product" {
            in_table = true;
            continue;
        }
        if !in_table || name.starts_with('-') {
            continue;
        }
        let Some(kind) = cells.get(1) else {
            return Err(format!("the clean-room rule's table has a row with no kind on line {}", number + 1));
        };
        if name.is_empty() || kind.is_empty() {
            return Err(format!("the clean-room rule's table has an empty cell on line {}", number + 1));
        }
        out.push(Prohibited { name: (*name).to_string(), kind: (*kind).to_string() });
    }
    Ok(out)
}

/// Whether `haystack` names `name`, as a whole word, whatever the case.
///
/// A whole word, because a product's name written without its space is not the
/// product's name: `Zorbix 900` is not `Zorbix900`. The case does not matter,
/// because a product's name in a file name or a string is written however the
/// writer felt like writing it.
pub fn mentions(haystack: &str, name: &str) -> bool {
    if name.is_empty() {
        return false;
    }
    let haystack = haystack.to_lowercase();
    let name = name.to_lowercase();
    let bytes = haystack.as_bytes();
    let mut at = 0usize;
    while at < haystack.len() {
        let Some(found) = haystack[at..].find(&name) else { break };
        let start = at + found;
        let end = start + name.len();
        let before = start == 0 || !is_word_byte(bytes[start - 1]);
        let after = end >= bytes.len() || !is_word_byte(bytes[end]);
        if before && after {
            return true;
        }
        at = end;
    }
    false
}

/// A byte that is part of an identifier.
fn is_word_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

/// Whether a **path** names `name`.
///
/// A path cannot hold a space in a Rust file name, so a two-word product's
/// name is written with a dash or an underscore: `zorbix-900.rs`. The name is
/// therefore checked in three forms.
fn path_mentions(path: &str, name: &str) -> bool {
    if mentions(path, name) {
        return true;
    }
    for written in [name.replace(' ', "-"), name.replace(' ', "_")] {
        if written != name && mentions(path, &written) {
            return true;
        }
    }
    false
}

/// Whether a file is prose.
///
/// Prose may name a product: the rule names them, `AGENTS.md` names them, and
/// a note about why something is the way it is may name one. Everything else —
/// source, tests, fixtures, configuration, a file name — may not.
pub fn is_prose(path: &Path) -> bool {
    path.extension().and_then(|e| e.to_str()) == Some("md")
}

/// The places in the tree that name a prohibited product.
///
/// Each hit is a file, a line and the name it found.
pub fn inspect(root: &Path) -> Result<Vec<String>, String> {
    let rule = root.join("docs/clean-room.md");
    let text = crate::tree::read_text(&rule)
        .map_err(|e| format!("could not read the clean-room rule at {}: {e}", rule.display()))?;
    let products = prohibited(&text)?;
    if products.is_empty() {
        return Err(
            "the clean-room rule names no products at all: a rule with no list checks nothing".to_string()
        );
    }

    let mut files = Vec::new();
    crate::tree::walk(root, root, &mut files, &|path| !is_prose(path));
    let mut hits = Vec::new();
    for relative in files {
        let shown = relative.to_string_lossy().replace('\\', "/");
        for product in &products {
            if path_mentions(&shown, &product.name) {
                hits.push(format!(
                    "{shown}: the file is named after `{}` — the tree may not carry another \
                     product's material",
                    product.name
                ));
            }
        }
        let path = root.join(&relative);
        // Every file in the tree, and every byte of it: a Rust file is read
        // with a subject, and the rest are read as bytes. §205.
        //
        // The extension is compared as a **string**: `Path::ends_with` asks
        // whether the path ends with a *component*, and `.rs` is not one.
        let is_rust = relative.to_string_lossy().ends_with(".rs");
        let text = if is_rust {
            crate::tree::read_rust(&path, crate::tree::Subject::Everything).ok()
        } else {
            crate::tree::read_bytes(&path).ok().map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
        };
        let Some(text) = text else { continue };
        for (number, line) in text.lines().enumerate() {
            for product in &products {
                if mentions(line, &product.name) {
                    hits.push(format!(
                        "{shown}:{}: `{}` is named here — the tree may not carry another \
                         product's material",
                        number + 1,
                        product.name
                    ));
                }
            }
        }
    }
    hits.sort();
    Ok(hits)
}

/// Run the gate.
pub fn run(root: &Path, _args: &[String]) -> Result<crate::Outcome, String> {
    crate::step("cleanroom");
    let hits = inspect(root)?;
    if hits.is_empty() {
        println!("   ok: no product's name is in the tree outside prose");
        Ok(crate::Outcome::Ran)
    } else {
        let mut problems = String::new();
        for hit in &hits {
            problems.push_str(&format!("   • {hit}\n"));
        }
        Err(format!(
            "{} clean-room problem(s):\n{problems}\n\
             The rule is in docs/clean-room.md. A product's name may appear in prose — in the\n\
             rule itself, in AGENTS.md, in a note about why something is the way it is — and\n\
             nowhere else: not in source, not in a test, not in a fixture, not in a file name.\n\
             If a name is here because the tree compares itself with that product, the\n\
             comparison is the thing to remove.",
            hits.len()
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    /// A tree on disk, removed when it is dropped.
    struct Temp {
        path: PathBuf,
    }

    impl Temp {
        fn new(name: &str) -> Self {
            let path = std::env::temp_dir().join(format!("craftkit-cleanroom-{}-{name}", std::process::id()));
            let _ = std::fs::remove_dir_all(&path);
            std::fs::create_dir_all(&path).expect("the temporary tree");
            Temp { path }
        }

        fn write(&self, relative: &str, body: &str) {
            let path = self.path.join(relative);
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).expect("the parent directory");
            }
            std::fs::write(&path, body).expect("the file");
        }

        fn path(&self) -> &Path {
            &self.path
        }
    }

    impl Drop for Temp {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }

    /// A rule document with the table the gate reads.
    ///
    /// The names are invented, so this file names no product: the gate scans
    /// the tree it lives in, and a file that has to prove the gate works has to
    /// be able to name what it looks for without being the thing it forbids.
    /// The real names are in `docs/clean-room.md`, and the test at the end
    /// checks the real tree against the real rule.
    fn rule() -> String {
        "| Product | Kind | Why it is here |\n\
         |---|--:|---|\n\
         | Zorbix 900 | commercial | a product that does not exist |\n\
         | Quibble Draw | copyleft | another one that does not exist |\n"
            .to_string()
    }

    #[test]
    fn the_table_is_read_from_the_rule() {
        let products = prohibited(&rule()).expect("a table");
        assert_eq!(products.len(), 2, "{products:?}");
        assert_eq!(products[0].name, "Zorbix 900", "{products:?}");
        assert_eq!(products[1].kind, "copyleft", "{products:?}");
    }

    #[test]
    fn a_document_with_no_table_names_no_products() {
        assert!(prohibited("# A rule\n\nNothing here.\n").expect("no table").is_empty());
    }

    #[test]
    fn a_row_with_an_empty_cell_is_a_broken_rule() {
        let text = "| Product | Kind | Why |\n|---|--:|---|\n|  | commercial | why |\n";
        assert!(prohibited(text).is_err());
    }

    #[test]
    fn a_product_name_in_a_source_file_is_reported_with_its_line() {
        let temp = Temp::new("source");
        temp.write("docs/clean-room.md", &rule());
        temp.write("crates/core/src/lib.rs", "pub fn f() {}\n// a Zorbix 900 export\n");
        let hits = inspect(temp.path()).expect("inspects");
        assert_eq!(hits.len(), 1, "{hits:?}");
        assert!(hits[0].contains("crates/core/src/lib.rs:2"), "{hits:?}");
        assert!(hits[0].contains("Zorbix 900"), "{hits:?}");
    }

    #[test]
    fn a_product_name_in_prose_is_not_reported() {
        let temp = Temp::new("prose");
        temp.write("docs/clean-room.md", &rule());
        temp.write("docs/design/notes.md", "We do not do what Zorbix 900 does.\n");
        assert!(inspect(temp.path()).expect("inspects").is_empty());
    }

    #[test]
    fn a_product_name_in_a_file_name_is_reported() {
        let temp = Temp::new("filename");
        temp.write("docs/clean-room.md", &rule());
        temp.write("crates/core/src/quibble-draw.rs", "pub fn f() {}\n");
        let hits = inspect(temp.path()).expect("inspects");
        assert_eq!(hits.len(), 1, "{hits:?}");
        assert!(hits[0].contains("the file is named after"), "{hits:?}");
    }

    #[test]
    fn a_name_that_is_not_the_products_name_is_not_the_product() {
        let temp = Temp::new("near");
        temp.write("docs/clean-room.md", &rule());
        temp.write("crates/core/src/lib.rs", "// Zorbix900 and Zorbix 9000\n");
        assert!(inspect(temp.path()).expect("inspects").is_empty());
    }

    #[test]
    fn the_case_of_a_name_does_not_matter() {
        let temp = Temp::new("case");
        temp.write("docs/clean-room.md", &rule());
        temp.write("crates/core/src/lib.rs", "// zorbix 900\n");
        assert_eq!(inspect(temp.path()).expect("inspects").len(), 1);
    }

    #[test]
    fn a_rule_that_names_no_product_is_reported_rather_than_passed() {
        let temp = Temp::new("empty");
        temp.write("docs/clean-room.md", "# The rule\n\nNo table.\n");
        assert!(inspect(temp.path()).is_err());
    }

    #[test]
    fn a_missing_rule_is_reported_rather_than_passed() {
        let temp = Temp::new("missing");
        assert!(inspect(temp.path()).is_err());
    }

    #[test]
    fn this_repository_carries_no_product_name_outside_prose() {
        let root = crate::root().expect("the workspace root");
        let hits = inspect(&root).expect("inspects");
        assert!(hits.is_empty(), "{}", hits.join("\n"));
    }
}
