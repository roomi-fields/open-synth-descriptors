//! A small XML reader: elements, attributes, text, comments, the prolog and the document type.
//! Each element keeps the line of its opening tag, which an import names in its refusals. The
//! writer side is `escape`.

/// One element.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Node {
    pub name: String,
    pub attrs: Vec<(String, String)>,
    pub children: Vec<Node>,
    /// The text directly inside the element, the pieces joined and trimmed.
    pub text: String,
    /// The line of its opening tag, from 1.
    pub line: usize,
}

impl Node {
    pub fn attr(&self, name: &str) -> Option<&str> {
        self.attrs.iter().find(|(k, _)| k == name).map(|(_, v)| v.as_str())
    }

    pub fn kids<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a Node> {
        self.children.iter().filter(move |c| c.name == name)
    }

    pub fn kid(&self, name: &str) -> Option<&Node> {
        self.children.iter().find(|c| c.name == name)
    }
}

/// A well-formedness fault: the line and what is wrong.
#[derive(Debug)]
pub(crate) struct XmlFault {
    pub line: usize,
    pub rule: String,
}

fn fail<T>(line: usize, rule: impl Into<String>) -> Result<T, XmlFault> {
    Err(XmlFault { line, rule: rule.into() })
}

pub(crate) fn escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            _ => out.push(c),
        }
    }
    out
}

fn unescape(s: &str) -> String {
    if !s.contains('&') {
        return s.to_string();
    }
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(at) = rest.find('&') {
        out.push_str(&rest[..at]);
        rest = &rest[at..];
        let Some(end) = rest.find(';') else { break };
        let entity = &rest[1..end];
        let decoded = match entity {
            "lt" => Some('<'),
            "gt" => Some('>'),
            "quot" => Some('"'),
            "apos" => Some('\''),
            "amp" => Some('&'),
            _ => entity
                .strip_prefix("#x")
                .and_then(|h| u32::from_str_radix(h, 16).ok())
                .or_else(|| entity.strip_prefix('#').and_then(|d| d.parse().ok()))
                .and_then(char::from_u32),
        };
        match decoded {
            Some(c) => {
                out.push(c);
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

/// The root element of `text`.
pub(crate) fn parse(text: &str) -> Result<Node, XmlFault> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let b = text.as_bytes();
    let (mut i, mut line) = (0usize, 1usize);
    let mut stack: Vec<Node> = Vec::new();
    let mut root: Option<Node> = None;
    while i < b.len() {
        if b[i] != b'<' {
            let next = text[i..].find('<').map_or(b.len(), |n| i + n);
            let chunk = &text[i..next];
            if let Some(top) = stack.last_mut() {
                top.text.push_str(&unescape(chunk));
            } else if !chunk.trim().is_empty() {
                return fail(line, "text outside the root element");
            }
            line += chunk.matches('\n').count();
            i = next;
            continue;
        }
        let rest = &text[i..];
        let end_of = |pat: &str| rest.find(pat).map_or_else(|| fail(line, format!("`{pat}` is missing")), Ok);
        if rest.starts_with("<!--") {
            let e = end_of("-->")?;
            line += rest[..e].matches('\n').count();
            i += e + 3;
        } else if rest.starts_with("<![CDATA[") {
            let e = end_of("]]>")?;
            match stack.last_mut() {
                Some(top) => top.text.push_str(&rest[9..e]),
                None => return fail(line, "text outside the root element"),
            }
            line += rest[..e].matches('\n').count();
            i += e + 3;
        } else if rest.starts_with("<?") {
            let e = end_of("?>")?;
            line += rest[..e].matches('\n').count();
            i += e + 2;
        } else if rest.starts_with("<!") {
            // The document type, whose internal subset may hold `>` inside brackets.
            let mut depth = 0i32;
            let mut e = None;
            for (k, c) in rest.char_indices() {
                match c {
                    '[' => depth += 1,
                    ']' => depth -= 1,
                    '>' if depth <= 0 => {
                        e = Some(k);
                        break;
                    }
                    _ => {}
                }
            }
            let Some(e) = e else { return fail(line, "the declaration is not closed") };
            line += rest[..e].matches('\n').count();
            i += e + 1;
        } else if rest.starts_with("</") {
            let e = end_of(">")?;
            let name = rest[2..e].trim();
            let Some(node) = stack.pop() else { return fail(line, format!("closing tag {name} with nothing open")) };
            if node.name != name {
                return fail(line, format!("closing tag {name} for {}", node.name));
            }
            i += e + 1;
            match stack.last_mut() {
                Some(p) => p.children.push(node),
                None if root.is_none() => root = Some(node),
                None => return fail(line, "a second root element"),
            }
        } else {
            // An opening tag: its end is the first `>` outside a quoted value.
            let mut j = i + 1;
            let mut quote = 0u8;
            while j < b.len() && (quote != 0 || b[j] != b'>') {
                if quote == 0 && (b[j] == b'"' || b[j] == b'\'') {
                    quote = b[j];
                } else if quote != 0 && b[j] == quote {
                    quote = 0;
                }
                j += 1;
            }
            if j >= b.len() {
                return fail(line, "a tag is not closed");
            }
            let mut inner = &text[i + 1..j];
            let empty = inner.ends_with('/');
            if empty {
                inner = &inner[..inner.len() - 1];
            }
            let name_end = inner.find(char::is_whitespace).unwrap_or(inner.len());
            let mut node = Node { name: inner[..name_end].to_string(), line, ..Node::default() };
            if node.name.is_empty() {
                return fail(line, "a tag with no name");
            }
            let mut attrs = inner[name_end..].trim();
            while !attrs.is_empty() {
                let Some(eq) = attrs.find('=') else { return fail(line, "an attribute has no value") };
                let key = attrs[..eq].trim().to_string();
                let after = attrs[eq + 1..].trim_start();
                let Some(q) = after.chars().next().filter(|c| *c == '"' || *c == '\'') else {
                    return fail(line, "an attribute value is not quoted");
                };
                let Some(close) = after[1..].find(q) else { return fail(line, "an attribute value is not closed") };
                if node.attrs.iter().any(|(k, _)| *k == key) {
                    return fail(line, format!("the attribute {key} is given twice"));
                }
                node.attrs.push((key, unescape(&after[1..1 + close])));
                attrs = after[close + 2..].trim_start();
            }
            line += text[i..j].matches('\n').count();
            i = j + 1;
            if empty {
                match stack.last_mut() {
                    Some(p) => p.children.push(node),
                    None if root.is_none() => root = Some(node),
                    None => return fail(line, "a second root element"),
                }
            } else {
                stack.push(node);
            }
        }
    }
    if let Some(open) = stack.last() {
        return fail(open.line, format!("{} is never closed", open.name));
    }
    let Some(mut root) = root else { return fail(1, "the file holds no element") };
    fn trim(n: &mut Node) {
        n.text = n.text.trim().to_string();
        n.children.iter_mut().for_each(trim);
    }
    trim(&mut root);
    Ok(root)
}
