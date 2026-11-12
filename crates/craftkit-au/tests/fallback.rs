//! The **fallback** door: the module that stands in where there is no
//! shared library to open.
//!
//! A target with no loader has no unit, and the host says so — but it
//! can only say so if the fallback exports the **same names** as the
//! module it stands in for. A name the unix module has and the fallback
//! does not is a call site that does not compile, and the door is shut
//! for the whole target rather than for one call.
//!
//! So this test reads the crate's own source and compares the two
//! blocks: the one that opens a bundle, and the one that refuses.

use std::collections::BTreeSet;
use std::path::PathBuf;

/// The crate's own source.
fn source() -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/lib.rs");
    std::fs::read_to_string(&path).expect("the crate's own source is there")
}

/// The lines of one `mod foreign` block.
///
/// The block is a top-level module, so it ends at the first line that is
/// a `}` on its own: that is the end of the module and not the end of a
/// function inside it.
fn block(cfg: &str) -> Vec<String> {
    let source = source();
    let lines: Vec<&str> = source.lines().collect();
    let mut starts = Vec::new();
    for (index, line) in lines.iter().enumerate() {
        if line.trim() != "mod foreign {" {
            continue;
        }
        // The attribute above the module says which target it is for.
        let mut above = index;
        let mut attribute = String::new();
        while above > 0 {
            above -= 1;
            let trimmed = lines[above].trim();
            if trimmed.starts_with("#[cfg(") {
                attribute = trimmed.to_string();
                break;
            }
        }
        if attribute == cfg {
            starts.push(index);
        }
    }
    assert_eq!(starts.len(), 1, "there is one `{cfg}` module of `foreign`");
    let start = starts[0];
    let mut end = start;
    while end < lines.len() && lines[end] != "}" {
        end += 1;
    }
    assert!(end < lines.len(), "the module ends");
    lines[start..=end].iter().map(|line| line.to_string()).collect()
}

/// The **doors** a module has: the functions it lets the rest of the
/// crate call.
fn doors(lines: &[String]) -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    for line in lines {
        let trimmed = line.trim();
        let name = if let Some(rest) = trimmed.strip_prefix("pub extern \"C\" fn ") {
            rest
        } else if let Some(rest) = trimmed.strip_prefix("pub fn ") {
            rest
        } else {
            continue;
        };
        let end = name.find(['(', '<']).unwrap_or(name.len());
        found.insert(name[..end].to_string());
    }
    found
}

/// The fallback has the same doors as the module it stands in for.
///
/// This is the whole of the rule: a target with no shared library must
/// still **compile**, and the only way to know that without a browser
/// is to compare the two modules' names.
#[test]
fn the_fallback_has_the_same_doors_as_the_unix_module() {
    let unix = doors(&block("#[cfg(unix)]"));
    let fallback = doors(&block("#[cfg(not(unix))]"));
    assert_eq!(unix, fallback, "the fallback and the unix module have the same doors");
}

/// The doors are the ones the host calls: the door is not a shape, it
/// is a list of names.
#[test]
fn the_doors_include_the_ones_the_host_calls() {
    let fallback = doors(&block("#[cfg(not(unix))]"));
    for door in ["open", "close", "get_property", "render", "make_instance"] {
        assert!(fallback.contains(door), "the fallback has the door `{door}`");
    }
}
