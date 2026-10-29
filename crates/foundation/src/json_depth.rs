//! How deep a JSON text nests, and the limit the tree keeps.
//!
//! **A limit the tree owns, below the parser's own.** `serde_json` refuses a
//! text nested more than 128 containers deep, and it says so in its own
//! words — *"recursion limit exceeded"* — which reads like a malformed file
//! rather than a file that is too deep. §287: the tree's writer bounded the
//! **size** of a design file and the reader bounded the **depth**, and the
//! two limits lived in different crates, so the writer produced files the
//! tree's own reader refused.
//!
//! So the depth is a limit the tree owns ✓, it is **below** the parser's ✓
//! (so the tree's refusal comes first ✓, in the tree's own words ✓), and
//! one scanner measures it ✓ — the writer ✓ and the reader ✓ use the same
//! one ✓, so the two cannot answer differently ✓.

use crate::MAX_JSON_DEPTH;

/// How deep a JSON text nests.
///
/// The **maximum** nesting of objects and arrays ✓ — a text with no
/// containers is depth 0 ✓ — and a text that is not JSON at all still gets a
/// depth ✓, because the caller has a parser that will say so ✓.
///
/// Strings are skipped whole ✓, so a bracket inside a string is not a
/// container ✓, and an escaped quote does not end the string early ✓.
pub fn json_depth(text: &str) -> usize {
    let mut depth: usize = 0;
    let mut deepest: usize = 0;
    let mut in_string = false;
    let mut escaped = false;
    for byte in text.bytes() {
        if in_string {
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == b'"' {
                in_string = false;
            }
            continue;
        }
        match byte {
            b'"' => in_string = true,
            b'{' | b'[' => {
                depth += 1;
                if depth > deepest {
                    deepest = depth;
                }
            }
            b'}' | b']' => depth = depth.saturating_sub(1),
            _ => {}
        }
    }
    deepest
}

/// Whether a JSON text nests deeper than the tree's limit.
///
/// The one question the writer and the reader both ask ✓.
pub fn json_is_too_deep(text: &str) -> bool {
    json_depth(text) > MAX_JSON_DEPTH
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The depth is the depth.
    ///
    /// A flat text is depth 0 ✓, an object is depth 1 ✓, and an object
    /// inside an object is depth 2 ✓.
    #[test]
    fn the_depth_is_the_deepest_container() {
        assert_eq!(json_depth("{}"), 1);
        assert_eq!(json_depth(r#"{"a":1}"#), 1);
        assert_eq!(json_depth(r#"{"a":{"b":1}}"#), 2);
        assert_eq!(json_depth(r#"{"a":[{"b":1}]}"#), 3);
        assert_eq!(json_depth("1"), 0);
        assert_eq!(json_depth(""), 0);
    }

    /// A bracket inside a string is not a container.
    ///
    /// A design's comment may hold any text at all ✓, and a comment holding
    /// a bracket must not make the file look deeper than it is ✓.
    #[test]
    fn a_bracket_inside_a_string_is_not_a_container() {
        assert_eq!(json_depth(r#"{"comment":"[[[["}"#), 1);
        assert_eq!(json_depth(r#"{"comment":"a \" b"}"#), 1);
        assert_eq!(json_depth(r#"{"comment":"\\"}"#), 1);
    }

    /// The limit is the tree's own, and it is below the parser's.
    ///
    /// The parser refuses at 128 ✓; the tree's own limit is lower ✓, so the
    /// tree's refusal is the one that happens ✓ — in the tree's own words ✓.
    #[test]
    fn the_limit_is_below_the_parser_s_own() {
        // Asserted with locals so clippy does not fold them into a constant
        // comparison -- the same trick the model's limit test uses.
        let (limit, parser) = (MAX_JSON_DEPTH, 128);
        assert!(limit < parser, "the tree's limit is {limit} and the parser's is {parser}");
        assert!(limit > 16, "the tree's limit leaves no room for a document");
    }

    /// A text over the limit is too deep, and one under it is not.
    #[test]
    fn the_limit_answers_the_one_question() {
        let deep = format!("{}1{}", "[".repeat(MAX_JSON_DEPTH + 1), "]".repeat(MAX_JSON_DEPTH + 1));
        assert!(json_is_too_deep(&deep), "a text over the limit is not too deep");
        let shallow = format!("{}1{}", "[".repeat(MAX_JSON_DEPTH), "]".repeat(MAX_JSON_DEPTH));
        assert!(!json_is_too_deep(&shallow), "a text at the limit is too deep");
    }
}
