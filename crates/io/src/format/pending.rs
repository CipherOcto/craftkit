//! Pending format changes, one file each.
//!
//! # The rule
//!
//! A change that alters what is saved **adds one file in `format/pending/`**. It
//! does not edit [`crate::format::CURRENT`], it does not edit the reader, and it
//! does not touch an already-released version. The release folds everything in
//! this directory into the next version and empties it.
//!
//! # The name
//!
//! The file, the `FILES` entry and the `mod` are **the same word**, in
//! `snake_case`: `mute_flag_to_node_field.rs`, `"mute_flag_to_node_field"` and
//! `mod mute_flag_to_node_field;`.
//!
//! Three documents used to say three different things here — the recipe said
//! the file was snake-case and the table entry kebab-case, the pattern
//! document's example used a name that is not a Rust identifier at all, and
//! the test that checked the name allowed only kebab-case — and the result was
//! that **no working change could satisfy the tree**: a name Rust accepts was
//! refused by the test, and a name the test accepted could not be compiled.
//! §81 has the measurements.
//!
//! # Which changes a file still needs
//!
//! A file records the changes that had been applied to it when it was written
//! ([`crate::format::Meta::pending`]), and a reader applies the changes this
//! build has **that the file has not had**. Without that, the reader applies a
//! change to a file this build wrote — and the file does not read back as it
//! was written.
//!
//! # Which changes a file has had that this build does not have
//!
//! The same list, read the other way, is the one the reader used to ignore.
//! A file that names a change this build has never heard of was written by a
//! build whose shape this build does not know: the document in it is in that
//! change's "after" shape, and the parts this build does not recognise are
//! dropped without a word. A reader that only ever subtracts the file's list
//! from its own cannot tell that file from one it understands. §95.
//!
//! [`unknown_names`] is that other direction, and the reader refuses a file
//! that needs it.
//!
//! # Why it is worth the ceremony
//!
//! The alternative — bump the number and edit the reader in one diff — reads
//! better right up until two people do it at once, at which point one of them
//! has to resolve a conflict in the one file everybody edits, and the losing
//! change silently disappears. One file per change cannot conflict that way,
//! and the release script becomes a `mv` instead of a judgement call.
//!
//! # What a file contains
//!
//! A `//!` note saying what changes and why, plus `pub fn step(doc: &mut
//! Document)` rewriting a document written by the *previous* version into the
//! shape this one expects. A change that needs no rewrite — a new optional
//! field, say — says so and leaves `step` out.
//!
//! # The example
//!
//! There is nothing pending at version 1. The recipe for adding one, and the
//! release script that folds them in, are in `docs/patterns/format-changes.md`.

/// Every pending change, sorted.
///
/// Sorted at runtime as well as written in order, so a mistake in the table
/// below is caught by a test rather than by a release.
pub fn names() -> Vec<&'static str> {
    let mut v = FILES.to_vec();
    v.sort_unstable();
    v
}

/// One entry per file in `format/pending/`.
///
/// Add a line here **and** a `mod <name>;` when you add a file. The
/// `the_pending_list_is_sorted` test keeps the table honest.
const FILES: &[&str] = &[];

/// The changes a file still needs: the ones this build has that the file has
/// not had.
///
/// The pure part of [`apply`], and the part a test can exercise without a
/// pending change existing — which is what §81 asked for.
fn missing<'a>(names: &[&'a str], already: &[String]) -> Vec<&'a str> {
    names.iter().copied().filter(|name| !already.iter().any(|a| a == name)).collect()
}

/// The changes a file names that this build has never heard of.
///
/// The other direction, and the one the reader used to be blind to: `missing`
/// works out what the file still **needs**, and this works out what the file
/// has **had** that this build does not have.
///
/// A name in this list means the file was written by a build whose shape this
/// build does not know. The document in such a file is in that change's
/// "after" shape, and the parts of it this build does not recognise are
/// dropped without a word — so a save writes the file back without them, and
/// the data is gone. §95.
///
/// The rule is symmetric with [`missing`] on purpose: one asks what the file
/// still needs, the other asks what it has had, and a reader that asks only
/// one of the two questions can read a file it does not understand.
pub fn unknown_names(already: &[String], known: &[&str]) -> Vec<String> {
    already.iter().filter(|name| !known.contains(&name.as_str())).cloned().collect()
}

/// A pending change and the step that applies it.
///
/// The name is the change's, and it is what a file records in its header.
pub type Step = (&'static str, fn(&mut craftkit_model::Document));

/// The steps this build has, in order.
///
/// Empty between releases, which is the normal state.
pub fn steps() -> Vec<Step> {
    names().into_iter().filter_map(|name| step_for(name).map(|step| (name, step))).collect()
}

/// Applying every pending step, oldest first.
///
/// `step` functions run in order, each seeing what the previous one produced.
/// An empty list is the normal state between releases, and this returns the
/// document untouched.
///
/// `already` is the list of changes the file has had: the ones it records in
/// its header. A change in that list is not run again — the file already has
/// it, and running it a second time would rewrite a file this build wrote.
pub fn apply(doc: &mut craftkit_model::Document, already: &[String]) {
    apply_steps(doc, already, &steps());
}

/// Applying the given steps, skipping the ones the file has had.
///
/// The seam that lets a test drive the reader with a step of its own: the
/// reader's rule — *only the changes a file has not had* — is otherwise
/// untestable until a change exists. §81.
pub fn apply_steps(doc: &mut craftkit_model::Document, already: &[String], steps: &[Step]) {
    let names: Vec<&str> = steps.iter().map(|(name, _)| *name).collect();
    let missing = missing(&names, already);
    let mut to_run: Vec<fn(&mut craftkit_model::Document)> = Vec::new();
    for (name, step) in steps {
        if missing.contains(name) {
            to_run.push(*step);
        }
    }
    run_steps(doc, &to_run);
}

/// The step for a pending change, if it has one.
///
/// Each pending module's `step` is listed here as it is added:
///
/// ```text
/// match name {
///     "example_name" => Some(example_name::step),
///     _ => None,
/// }
/// ```
///
/// (It is a comment rather than a `match` with one arm, because a `match`
/// with one arm is a lint, and the arm it will grow into is not there yet.)
fn step_for(name: &str) -> Option<fn(&mut craftkit_model::Document)> {
    let _ = name;
    None
}

/// Run the steps, in order, each seeing what the previous one produced.
///
/// The order is the contract: a step rewrites a document written by the
/// *previous* version, so a step that ran before its predecessor would see
/// the wrong shape and rewrite the wrong thing.
///
/// §45 is the finding: the list has been empty since version 1, so this ran
/// exactly zero times, and nothing tested the order or the chaining. The
/// first real format change would have been the first execution.
fn run_steps(doc: &mut craftkit_model::Document, steps: &[fn(&mut craftkit_model::Document)]) {
    for step in steps {
        step(doc);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_pending_list_is_sorted() {
        let names = names();
        let mut sorted = names.clone();
        sorted.sort_unstable();
        assert_eq!(names, sorted, "keep `FILES` sorted so a release is a `mv`");
    }

    #[test]
    fn applying_no_steps_leaves_the_document_alone() {
        // The empty case, deliberately: "no steps" is a fact about the list,
        // not about the document. Before §81 this test called `apply`, so it
        // failed the moment a change was pending — the test was really saying
        // "nothing may be pending", which is not a property of the code.
        let mut doc = craftkit_model::Document::empty("t");
        let _ = doc.add_node("A", craftkit_model::NodeKind::Source);
        let before = doc.clone();
        run_steps(&mut doc, &[]);
        assert_eq!(doc, before);
    }

    #[test]
    fn a_file_that_has_had_a_change_is_not_had_it_again() {
        // The rule that keeps a file this build wrote from being rewritten:
        // the changes it records are not applied to it a second time.
        //
        // The names below are synthetic, and the rule is about the *filter*,
        // so the test does not need a pending change to exist to mean
        // something. §81: before this, the reader applied every pending step
        // to every file, including the files it had just written.
        let names = ["a_change", "another_change"];
        let none: Vec<String> = Vec::new();
        assert_eq!(missing(&names, &none), vec!["a_change", "another_change"]);

        let one = vec!["a_change".to_string()];
        assert_eq!(missing(&names, &one), vec!["another_change"]);

        let both = vec!["a_change".to_string(), "another_change".to_string()];
        assert!(missing(&names, &both).is_empty(), "a file that has had every change needs none");

        // A change this build does not have is not the file's business.
        let other = vec!["a_change_from_another_build".to_string()];
        assert_eq!(missing(&names, &other).len(), 2);
    }

    /// The other direction: the changes a file has had that this build does
    /// not have.
    ///
    /// §95: the reader worked out what a file still needed and never asked
    /// what it had already had, so a file written by a build with a format
    /// change this build does not have was read like any other — and the
    /// parts of the document that change had put there were dropped without a
    /// word.
    #[test]
    fn the_changes_a_file_has_that_this_build_does_not_have_are_worked_out() {
        let known = ["a_change", "another_change"];
        let none: Vec<String> = Vec::new();
        assert!(unknown_names(&none, &known).is_empty(), "a file that has had nothing is fine");

        let one = vec!["a_change".to_string()];
        assert!(unknown_names(&one, &known).is_empty(), "a change this build has is not unknown");

        let stranger = vec!["a_change_from_another_build".to_string()];
        assert_eq!(
            unknown_names(&stranger, &known),
            vec!["a_change_from_another_build".to_string()],
            "a change this build has never heard of is the one that matters"
        );

        let both = vec!["a_change".to_string(), "a_stranger".to_string()];
        assert_eq!(unknown_names(&both, &known), vec!["a_stranger".to_string()]);
    }

    #[test]
    fn the_steps_run_in_order_and_each_sees_the_previous_one() {
        // The list has been empty since version 1, so `apply` had never run
        // and the order it promises was never exercised. The steps below are
        // synthetic: they are not pending changes, they are two ordinary
        // functions, and the contract they demonstrate is the one the module
        // documents — "step functions run in order, each seeing what the
        // previous one produced".
        fn rename(doc: &mut craftkit_model::Document) {
            if let Some(node) = doc.nodes.first_mut() {
                node.name = "Renamed".to_string();
            }
        }
        fn expects_the_rename(doc: &mut craftkit_model::Document) {
            let name = doc.nodes.first().map(|n| n.name.clone()).unwrap_or_default();
            assert_eq!(name, "Renamed", "the second step did not see the first one's work");
        }

        let mut doc = craftkit_model::Document::empty("t");
        let _ = doc.add_node("A", craftkit_model::NodeKind::Source);
        run_steps(&mut doc, &[rename, expects_the_rename]);
        assert_eq!(doc.nodes.first().map(|n| n.name.clone()).unwrap_or_default(), "Renamed");
    }

    #[test]
    fn the_steps_run_in_the_order_they_are_given() {
        // Two steps that both write, and the last one wins — which is what
        // "in order" means when a step sees what the previous one produced.
        // This is what makes the order a contract rather than an accident of
        // how the list happens to be written.
        fn first(doc: &mut craftkit_model::Document) {
            if let Some(node) = doc.nodes.first_mut() {
                node.name = "First".to_string();
            }
        }
        fn second(doc: &mut craftkit_model::Document) {
            if let Some(node) = doc.nodes.first_mut() {
                node.name = "Second".to_string();
            }
        }

        let mut doc = craftkit_model::Document::empty("t");
        let _ = doc.add_node("A", craftkit_model::NodeKind::Source);
        run_steps(&mut doc, &[first, second]);
        assert_eq!(doc.nodes.first().map(|n| n.name.clone()).unwrap_or_default(), "Second");
    }

    #[test]
    fn the_files_on_disk_and_the_table_agree() {
        // The failure this catches: someone writes the migration and does not
        // register it. `every_pending_file_is_wired_into_apply` cannot see that
        // — it asserts the table is empty, so it stays green while a file sits
        // in `pending/` that `names()` never returns and `apply` never calls.
        // The design then loads without its rewrite, which is not an error, it
        // is wrong values.
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/format/pending");

        // A fresh clone has no `pending/` at all, because git does not track an
        // empty directory. That is a real state and an empty table is right for
        // it, so a missing directory is empty rather than an error.
        let mut on_disk: Vec<String> = match std::fs::read_dir(&dir) {
            Ok(entries) => entries
                .flatten()
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .filter(|n| n.ends_with(".rs") && n != "mod.rs" && n != "lib.rs")
                .map(|n| n.trim_end_matches(".rs").to_string())
                .collect(),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Vec::new(),
            Err(e) => panic!("could not read {}: {e}", dir.display()),
        };
        on_disk.sort();

        let listed: Vec<String> = names().iter().map(|n| (*n).to_string()).collect();
        assert_eq!(
            listed, on_disk,
            "the pending changes on disk and the `FILES` table disagree.\n\
             Every file in `crates/io/src/format/pending/` needs a `FILES` entry here\n\
             and a `mod <name>;` beside it. A file that is not listed has its `step`\n\
             never called, so an older design loads without its rewrite — silently,\n\
             with no error. `names()` returned {listed:?}; the directory holds {on_disk:?}."
        );
    }

    #[test]
    fn every_pending_file_is_wired_into_apply() {
        // Without this, adding a pending change and forgetting to call its
        // `step` would leave the rewrite unwritten and this module quietly
        // doing nothing forever.
        //
        // §81: this used to assert that the table was **empty** — so it fired
        // the moment a change was pending, which is the moment it was written
        // to protect, and its advice ("call each one's `step` from `apply`")
        // was advice the person had already taken. It asks the question now:
        // does each listed change have a step?
        for name in names() {
            assert!(
                step_for(name).is_some(),
                "pending/{name}.rs is listed but `apply` never calls its `step`, so the \
                 migration does not run and an older design loads without its rewrite"
            );
        }
    }

    #[test]
    fn every_pending_name_is_a_plausible_file_name() {
        // The name is a Rust module name, so it has to be one: the file is
        // `pending/<name>.rs`, the table entry is `<name>` and the declaration
        // is `mod <name>;`. All three are the same word.
        //
        // §81: this allowed only lower-case letters, digits and dashes, and
        // said "snake-case" in the message. A dashed name is not a Rust
        // identifier, so the rule refused every name that could actually work,
        // and the recipe's own kebab-case table entry could not be compiled.
        for name in names() {
            assert!(!name.contains('/'), "a pending change is one file: {name}");
            assert!(
                name.chars().all(|c| c.is_ascii_lowercase() || c == '_' || c.is_ascii_digit()),
                "`{name}` should be a snake-case file name"
            );
            assert!(
                name.starts_with(|c: char| c.is_ascii_lowercase()),
                "`{name}` should start with a letter, so `mod {name};` is a module"
            );
        }
    }
}
