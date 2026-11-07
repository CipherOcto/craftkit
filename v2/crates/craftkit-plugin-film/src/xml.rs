//! A small **XML** reader.
//!
//! FCPXML is an XML text, and reading it needs a reader that knows
//! what an element is. This one is small and it is **strict about
//! what it accepts**: it reads elements, attributes and text, and it
//! refuses a text it does not understand rather than guessing.
//!
//! The rules:
//!
//! - a **declaration**, a **doctype** and a **comment** are skipped;
//! - an **attribute** value is in quotes, and the five entities of XML
//!   are read in it: `&amp;`, `&lt;`, `&gt;`, `&quot;` and
//!   `&apos;`;
//! - a text that is **not** XML is refused, with the place it is not;
//! - the **depth** of the text is bounded, and so is its length, so a
//!   text that is deep cannot make the reader deep in the stack.

/// The most elements may be **deep**.
pub(crate) const MAX_DEPTH: usize = 64;

/// The most characters a text may have.
pub(crate) const MAX_CHARS: usize = 4_000_000;

/// An **element**: a name, its attributes, its children and its text.
pub struct Element {
    /// The name of the element.
    pub name: String,
    /// The attributes, in the order they were read.
    pub attributes: Vec<(String, String)>,
    /// The children of the element.
    pub children: Vec<Element>,
    /// The text of the element, without the text of its children.
    pub text: String,
}

impl Element {
    /// The **attribute** of that name, when the element has it.
    pub fn attribute(&self, name: &str) -> Option<&str> {
        self.attributes.iter().find(|(key, _)| key == name).map(|(_, value)| value.as_str())
    }

    /// The **child** of that name, when the element has one.
    pub fn child(&self, name: &str) -> Option<&Element> {
        self.children.iter().find(|child| child.name == name)
    }

    /// The **children** of that name.
    pub fn children_named<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a Element> {
        self.children.iter().filter(move |child| child.name == name)
    }

    /// The **first** element of that name, anywhere below this one.
    pub fn find(&self, name: &str) -> Option<&Element> {
        self.find_all(name).into_iter().next()
    }

    /// Every element of that name, anywhere below this one.
    pub fn find_all(&self, name: &str) -> Vec<&Element> {
        let mut found = Vec::new();
        let mut stack: Vec<&Element> = self.children.iter().rev().collect();
        while let Some(element) = stack.pop() {
            if element.name == name {
                found.push(element);
            }
            for child in element.children.iter().rev() {
                stack.push(child);
            }
        }
        found
    }
}

/// Whether the text says what it says at that place.
fn starts_with(chars: &[char], at: usize, what: &str) -> bool {
    let mut cursor = at;
    for want in what.chars() {
        match chars.get(cursor) {
            Some(char) if *char == want => cursor += 1,
            _ => return false,
        }
    }
    true
}

/// Move past the **space** of the text.
fn skip_space(chars: &[char], at: &mut usize) {
    while matches!(chars.get(*at), Some(char) if char.is_whitespace()) {
        *at += 1;
    }
}

/// Move past the text up to what it says.
fn skip_until(chars: &[char], at: &mut usize, what: &str) {
    while *at < chars.len() {
        if starts_with(chars, *at, what) {
            *at += what.chars().count();
            return;
        }
        *at += 1;
    }
}

/// The **entities** of XML, read.
fn decode(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = rest.find('&') {
        out.push_str(&rest[..at]);
        rest = &rest[at..];
        let Some(end) = rest.find(';') else {
            out.push('&');
            rest = &rest[1..];
            continue;
        };
        let entity = &rest[1..end];
        let read = match entity {
            "amp" => Some('&'),
            "lt" => Some('<'),
            "gt" => Some('>'),
            "quot" => Some('"'),
            "apos" => Some('\''),
            _ => None,
        };
        match read {
            Some(char) => {
                out.push(char);
                rest = &rest[end + 1..];
            }
            None => {
                out.push('&');
                rest = &rest[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

/// The **name** of an element or of an attribute.
fn name_of(chars: &[char], at: &mut usize) -> String {
    let start = *at;
    while let Some(char) = chars.get(*at) {
        if char.is_whitespace() || *char == '=' || *char == '>' || *char == '/' || *char == '<' {
            break;
        }
        *at += 1;
    }
    chars[start..*at].iter().collect()
}

/// The **text** of an element, up to the next `<`.
fn text_of(chars: &[char], at: &mut usize) -> String {
    let start = *at;
    while let Some(char) = chars.get(*at) {
        if *char == '<' {
            break;
        }
        *at += 1;
    }
    decode(&chars[start..*at].iter().collect::<String>())
}

/// The **attributes** of an element.
fn attributes_of(chars: &[char], at: &mut usize) -> Result<(Vec<(String, String)>, bool), String> {
    let mut attributes = Vec::new();
    loop {
        skip_space(chars, at);
        match chars.get(*at) {
            Some('>') => {
                *at += 1;
                return Ok((attributes, false));
            }
            Some('/') if starts_with(chars, *at, "/>") => {
                *at += 2;
                return Ok((attributes, true));
            }
            None => return Err("the element is not finished".to_string()),
            _ => {}
        }
        let key = name_of(chars, at);
        if key.is_empty() {
            return Err("an attribute has a name".to_string());
        }
        skip_space(chars, at);
        if chars.get(*at) != Some(&'=') {
            return Err(format!("the attribute `{key}` has a value"));
        }
        *at += 1;
        skip_space(chars, at);
        let Some(quote) = chars.get(*at).copied() else {
            return Err(format!("the attribute `{key}` has a value"));
        };
        if quote != '"' && quote != '\'' {
            return Err(format!("the attribute `{key}` is in quotes"));
        }
        *at += 1;
        let start = *at;
        while let Some(char) = chars.get(*at) {
            if *char == quote {
                break;
            }
            *at += 1;
        }
        if chars.get(*at) != Some(&quote) {
            return Err(format!("the attribute `{key}` is not finished"));
        }
        let value: String = chars[start..*at].iter().collect();
        *at += 1;
        attributes.push((key, decode(&value)));
    }
}

/// The **element** at that place.
fn element_of(chars: &[char], at: &mut usize, depth: usize) -> Result<Element, String> {
    if depth > MAX_DEPTH {
        return Err(format!("the text is more than {MAX_DEPTH} elements deep"));
    }
    if chars.get(*at) != Some(&'<') {
        return Err("an element starts with `<`".to_string());
    }
    *at += 1;
    let name = name_of(chars, at);
    if name.is_empty() {
        return Err("an element has a name".to_string());
    }
    let (attributes, empty) = attributes_of(chars, at)?;
    if empty {
        return Ok(Element { name, attributes, children: Vec::new(), text: String::new() });
    }
    let mut children: Vec<Element> = Vec::new();
    let mut text = String::new();
    loop {
        match chars.get(*at) {
            None => return Err(format!("the element `{name}` is not finished")),
            Some('<') => {
                if starts_with(chars, *at, "</") {
                    *at += 2;
                    let closing = name_of(chars, at);
                    skip_space(chars, at);
                    if chars.get(*at) != Some(&'>') {
                        return Err(format!("the element `{name}` is not closed"));
                    }
                    *at += 1;
                    if closing != name {
                        return Err(format!("the element `{name}` is closed by `{closing}`"));
                    }
                    return Ok(Element { name, attributes, children, text: text.trim().to_string() });
                }
                if starts_with(chars, *at, "<!--") {
                    skip_until(chars, at, "-->");
                    continue;
                }
                if starts_with(chars, *at, "<?") {
                    skip_until(chars, at, "?>");
                    continue;
                }
                children.push(element_of(chars, at, depth + 1)?);
            }
            Some(_) => {
                text.push_str(&text_of(chars, at));
            }
        }
    }
}

/// **Read** an XML text and give the element at its root.
pub fn parse(text: &str) -> Result<Element, String> {
    if text.chars().count() > MAX_CHARS {
        return Err(format!("the text is more than {MAX_CHARS} characters long"));
    }
    let chars: Vec<char> = text.chars().collect();
    let mut at = 0usize;
    // The **declaration**, the **doctype** and the **comments** before
    // the root are skipped.
    loop {
        skip_space(&chars, &mut at);
        if starts_with(&chars, at, "<?") {
            skip_until(&chars, &mut at, "?>");
            continue;
        }
        if starts_with(&chars, at, "<!--") {
            skip_until(&chars, &mut at, "-->");
            continue;
        }
        if starts_with(&chars, at, "<!") {
            skip_until(&chars, &mut at, ">");
            continue;
        }
        break;
    }
    element_of(&chars, &mut at, 0)
}
