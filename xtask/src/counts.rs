//! `cargo xtask counts` — the counts of the tree, and the block that holds
//! them.
//!
//! The README says, in a dozen places, how many commands each battery
//! carries. Those numbers are **derived**: they are the size of the tree,
//! and the size of the tree is not something a human should keep by hand.
//!
//! This command reads the tree and prints the counts, and with `--write` it
//! writes them into the README's **counted block** — the one place where
//! the tree's nine numbers are written down together. The prose elsewhere
//! still says what a section's battery carried at the time; the block says
//! what the tree is now, and the block is **checked against the registry**
//! by the tree's own test.
//!
//! # Where the numbers come from
//!
//! **From the source, and the source is checked against the registry.** The
//! xtask cannot link the tree's own crates — a checker that cannot be built
//! when the tree does not compile is a checker that is silent exactly when
//! it is needed — so it reads the source as text. The tree's test compares
//! that reading with the **registry**, which is the truth, and fails when
//! they disagree. So the two are not two truths: one is checked against the
//! other.
//!
//! # The counted block
//!
//! ```text
//! <!-- the counts of the tree: begin -->
//! | battery | commands |
//! |---|---|
//! | grid | 188 |
//! <!-- the counts of the tree: end -->
//! ```
//!
//! The markers are there so that a reader — a person or a test — can find
//! the block without guessing, and so that writing it back is a **replace**
//! and not an append.

use std::path::Path;

use crate::step;

/// The marker that opens the counted block.
pub const BEGIN: &str = "<!-- the counts of the tree: begin -->";

/// The marker that closes the counted block.
pub const END: &str = "<!-- the counts of the tree: end -->";

/// The batteries the tree counts, and the crate of each.
///
/// The nine batteries the README counts. The other plugins — the picture,
/// the image, the typed value, the CLAP and VST3 batteries — are not
/// batteries the README counts, and they are not counted here either.
pub const BATTERIES: [&str; 9] = ["grid", "prose", "pdf", "film", "layout", "cad", "fx", "sound", "vector"];

/// The commands a crate has: the attributes that carry an id.
///
/// **The count of the text**, which is the count of the commands: the derive
/// writes one `#[command(` for each of them, and the tree's test compares
/// this number with the registry's.
pub fn commands_in(root: &Path, crate_name: &str) -> Result<usize, String> {
    let directory = root.join("crates").join(format!("craftkit-plugin-{crate_name}")).join("src");
    let entries =
        std::fs::read_dir(&directory).map_err(|e| format!("could not read {}: {e}", directory.display()))?;
    let mut total = 0;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|extension| extension.to_str()) != Some("rs") {
            continue;
        }
        let text = crate::tree::read_rust(&path, crate::tree::Subject::Everything)
            .map_err(|e| format!("could not read {}: {e}", path.display()))?;
        total += text.matches("#[command(").count();
    }
    Ok(total)
}

/// The counts of every battery, by name.
pub fn counts(root: &Path) -> Result<Vec<(String, usize)>, String> {
    let mut found = Vec::new();
    for battery in BATTERIES {
        found.push((battery.to_string(), commands_in(root, battery)?));
    }
    Ok(found)
}

/// The counted block, as the README should hold it.
pub fn the_block(root: &Path) -> Result<String, String> {
    let counts = counts(root)?;
    let mut block = String::from(BEGIN);
    block.push_str("\n| battery | commands |\n|---|---|\n");
    for (battery, count) in counts {
        block.push_str(&format!("| {battery} | {count} |\n"));
    }
    block.push_str(END);
    Ok(block)
}

/// The counted block as the README **holds** it.
///
/// `None` when the block is not there, or when the markers are the wrong way
/// round: a block with no markers is a block nobody can find.
pub fn the_block_in(readme: &str) -> Option<String> {
    let start = readme.find(BEGIN)?;
    let end = readme.find(END)?;
    if end < start {
        return None;
    }
    Some(readme[start..end + END.len()].to_string())
}

/// Replace the counted block in a README with a new one.
pub fn replace_block(readme: &str, block: &str) -> Result<String, String> {
    let start = readme
        .find(BEGIN)
        .ok_or_else(|| format!("the README has no counted block: the marker `{BEGIN}` is not in it"))?;
    let end = readme
        .find(END)
        .ok_or_else(|| format!("the README has no counted block: the marker `{END}` is not in it"))?;
    if end < start {
        return Err("the counted block's markers are the wrong way round".to_string());
    }
    let mut out = String::with_capacity(readme.len() + block.len());
    out.push_str(&readme[..start]);
    out.push_str(block);
    out.push_str(&readme[end + END.len()..]);
    Ok(out)
}

/// Run the command.
pub fn run(root: &Path, args: &[String]) -> Result<crate::Outcome, String> {
    let write = args.iter().any(|a| a == "--write");
    step("counts");
    let counts = counts(root)?;
    for (battery, count) in &counts {
        println!("   {battery}: {count}");
    }
    if !write {
        println!("   (the README's block is written with --write)");
        return Ok(crate::Outcome::Ran);
    }
    let block = the_block(root)?;
    let readme_path = root.join("README.md");
    let readme = crate::tree::read_text(&readme_path)
        .map_err(|e| format!("could not read {}: {e}", readme_path.display()))?;
    let updated = replace_block(&readme, &block)?;
    if updated == readme {
        println!("   the block is already right");
        return Ok(crate::Outcome::Ran);
    }
    std::fs::write(&readme_path, updated)
        .map_err(|e| format!("could not write {}: {e}", readme_path.display()))?;
    println!("   the block is written");
    // The write is a change to the tree, so the tree's gate is the check:
    // the counts test reads the block and the registry, and the gate runs it.
    Ok(crate::Outcome::Ran)
}
