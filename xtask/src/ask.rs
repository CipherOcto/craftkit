//! `cargo xtask ask` — one door for the tree's answers.
//!
//! # The question this answers
//!
//! `cargo xtask find` answers a question it knows how to ask: *where does
//! this command live*, *what is in this area*, *what is this bound*. An
//! agent that does not know which of those its question is has to **search**
//! — and the survey measured that at 16 greps an arc, and the greps are the
//! half of the arc that finds out what the tree already knows.
//!
//! This command takes the **words** and answers with everything the tree
//! says about them, wherever it says it:
//!
//! ```text
//! cargo xtask ask pivot live
//! cargo xtask ask "the format of a field"
//! ```
//!
//! Every answer names the **place**: the file and the line, or the section.
//! A name without a place is a name you look up again.

use std::path::Path;

use crate::index::Index;

/// Run the command.
pub fn run(root: &Path, args: &[String]) -> Result<crate::Outcome, String> {
    let words: Vec<String> = args
        .iter()
        .flat_map(|arg| arg.split_whitespace())
        .map(|word| word.to_lowercase())
        .filter(|word| !word.is_empty())
        .collect();
    if words.is_empty() {
        return Err("the question needs words: `cargo xtask ask pivot live`".to_string());
    }
    let index = crate::index::build(root).map_err(|e| format!("the tree could not be indexed: {e}"))?;
    let readme = crate::tree::read_text(root.join("README.md"))
        .map_err(|e| format!("could not read the README: {e}"))?;
    let mut out = String::new();
    out.push_str(&the_commands(&index, &words));
    out.push_str(&the_bounds(&index, &words));
    out.push_str(&the_sections(&readme, &words));
    out.push_str(&the_ports(&words));
    if out.is_empty() {
        println!("nothing in the tree says that");
        return Ok(crate::Outcome::Ran);
    }
    print!("{out}");
    Ok(crate::Outcome::Ran)
}

/// Whether a text says **one of** the words.
///
/// The commands, the bounds and the ports are searched **loosely**: a reader
/// who asks for "pivot live" wants everything about pivots, and the tree
/// answers with everything it has.
fn says(text: &str, words: &[String]) -> bool {
    let text = text.to_lowercase();
    words.iter().any(|word| text.contains(word))
}

/// Whether a text says **all** of the words.
///
/// The sections are searched **precisely**: a section that mentions one word
/// of the question is not an answer to it, and a table of 160 sections that
/// merely mention "live" is a table nobody reads.
fn says_all(text: &str, words: &[String]) -> bool {
    let text = text.to_lowercase();
    words.iter().all(|word| text.contains(word))
}

/// The **commands** that say the words.
fn the_commands(index: &Index, words: &[String]) -> String {
    let mut out = String::new();
    let mut count = 0;
    for command in &index.commands {
        let hit = says(&command.id, words)
            || command.label.as_deref().is_some_and(|label| says(label, words))
            || command.doc.as_deref().is_some_and(|doc| says(doc, words));
        if !hit {
            continue;
        }
        count += 1;
        out.push_str(&format!(
            "   {id:<44} {area:<10} {place}\n",
            id = command.id,
            area = command.battery,
            place = command.where_(),
        ));
    }
    if count == 0 {
        return out;
    }
    let mut header = format!("── the commands ({count})\n");
    header.push_str(&out);
    header
}

/// The **bounds** that say the words.
fn the_bounds(index: &Index, words: &[String]) -> String {
    let mut out = String::new();
    let mut count = 0;
    for bound in &index.bounds {
        if !says(&bound.name, words) && !says(&bound.value, words) {
            continue;
        }
        count += 1;
        out.push_str(&format!(
            "   {:<40} {:<12} {}:{}\n",
            bound.name,
            bound.value,
            bound.file.display(),
            bound.line
        ));
    }
    if count == 0 {
        return out;
    }
    let mut header = format!("── the bounds ({count})\n");
    header.push_str(&out);
    header
}

/// The **sections** of the README that say the words.
fn the_sections(readme: &str, words: &[String]) -> String {
    let mut found: Vec<(usize, String)> = Vec::new();
    let mut number: Option<usize> = None;
    let mut title = String::new();
    let mut body = String::new();
    for line in readme.lines() {
        if let Some(rest) = line.strip_prefix("## ") {
            if let Some(section) = number
                && (says_all(&title, words) || says_all(&body, words))
            {
                found.push((section, title.clone()));
            }
            number = None;
            title.clear();
            body.clear();
            if let Some((text, name)) = rest.split_once('.')
                && let Ok(parsed) = text.trim().parse::<usize>()
            {
                number = Some(parsed);
                title = name.trim().to_string();
            }
            continue;
        }
        body.push_str(line);
        body.push('\n');
    }
    if let Some(section) = number
        && (says_all(&title, words) || says_all(&body, words))
    {
        found.push((section, title.clone()));
    }
    if found.is_empty() {
        return String::new();
    }
    let mut out = format!("── the sections ({})\n", found.len());
    for (section, title) in found {
        out.push_str(&format!("   §{section:<8} {title}\n"));
    }
    out
}

/// The **ports** that say the words.
fn the_ports(words: &[String]) -> String {
    let mut out = String::new();
    let mut count = 0;
    for port in crate::ports::PORTS {
        if !says(port.rule, words) && !says(port.from, words) {
            continue;
        }
        count += 1;
        out.push_str(&format!("   {:<10} {:<52} {}\n", port.section, port.rule, port.from));
    }
    if count == 0 {
        return out;
    }
    let mut header = format!("── the ports ({count})\n");
    header.push_str(&out);
    header
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_text_says_the_words() {
        assert!(says("the way a pivot shows its value", &["pivot".to_string()]));
        assert!(!says("the way a pivot shows its value", &["sound".to_string()]));
    }

    #[test]
    fn a_word_is_a_word_and_not_a_letter() {
        assert!(says("SheetProtection", &["protection".to_string()]));
        assert!(!says("SheetProtection", &["sound".to_string()]));
    }

    #[test]
    fn a_section_says_the_question_only_when_it_says_all_of_it() {
        let words = vec!["pivot".to_string(), "live".to_string()];
        assert!(says_all("the pivot shows its value live", &words));
        assert!(!says_all("the pivot shows its value", &words));
    }
}
