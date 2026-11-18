//! A small XML reader and writer for the tests: elements, attributes, text, comments, the
//! prolog and the document type. It keeps the line of each element, which an import names in
//! its refusals.

#![allow(dead_code)]

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Node {
    pub name: String,
    pub attrs: Vec<(String, String)>,
    pub children: Vec<Node>,
    /// The text directly inside the element, the pieces joined.
    pub text: String,
    /// The line of its opening tag, from 1.
    pub line: usize,
}

/// What a document holds: its document type declaration (the text between `<!DOCTYPE` and
/// `>`) and its root element.
#[derive(Clone, Debug)]
pub struct Document {
    pub doctype: Option<String>,
    pub root: Node,
}

impl Node {
    pub fn new(name: &str, attrs: &[(&str, &str)]) -> Node {
        Node { name: name.into(), attrs: attrs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect(), ..Node::default() }
    }

    pub fn with(mut self, child: Node) -> Node {
        self.children.push(child);
        self
    }

    pub fn attr(&self, name: &str) -> Option<&str> {
        self.attrs.iter().find(|(k, _)| k == name).map(|(_, v)| v.as_str())
    }

    /// The direct children named `name`.
    pub fn kids(&self, name: &str) -> Vec<&Node> {
        self.children.iter().filter(|c| c.name == name).collect()
    }

    pub fn kid(&self, name: &str) -> Option<&Node> {
        self.children.iter().find(|c| c.name == name)
    }

    /// Every descendant named `name`, in document order.
    pub fn all(&self, name: &str) -> Vec<&Node> {
        let mut out = Vec::new();
        fn walk<'a>(n: &'a Node, name: &str, out: &mut Vec<&'a Node>) {
            for c in &n.children {
                if c.name == name {
                    out.push(c);
                }
                walk(c, name, out);
            }
        }
        walk(self, name, &mut out);
        out
    }

    /// The element at a path of child indices.
    pub fn at_mut(&mut self, path: &[usize]) -> &mut Node {
        let mut n = self;
        for i in path {
            n = &mut n.children[*i];
        }
        n
    }

    /// The paths of child indices of this element and of every descendant, in document order.
    pub fn paths(&self) -> Vec<Vec<usize>> {
        let mut out = vec![vec![]];
        for (i, c) in self.children.iter().enumerate() {
            for mut p in c.paths() {
                p.insert(0, i);
                out.push(p);
            }
        }
        out
    }

    /// The element written back, one tag per line.
    pub fn to_xml(&self) -> String {
        let mut out = String::new();
        self.write(&mut out, 0);
        out
    }

    fn write(&self, out: &mut String, depth: usize) {
        let pad = "  ".repeat(depth);
        out.push_str(&format!("{pad}<{}", self.name));
        for (k, v) in &self.attrs {
            out.push_str(&format!(" {k}=\"{}\"", escape(v)));
        }
        if self.children.is_empty() && self.text.is_empty() {
            out.push_str("/>\n");
        } else if self.children.is_empty() {
            out.push_str(&format!(">{}</{}>\n", escape(&self.text), self.name));
        } else {
            out.push_str(">\n");
            for c in &self.children {
                c.write(out, depth + 1);
            }
            out.push_str(&format!("{pad}</{}>\n", self.name));
        }
    }
}

pub fn escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

fn unescape(s: &str) -> String {
    s.replace("&lt;", "<").replace("&gt;", ">").replace("&quot;", "\"").replace("&apos;", "'").replace("&amp;", "&")
}

/// A document with its prolog and the public document type of MIDNAM, for a root element.
pub fn document_text(root: &Node) -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!DOCTYPE MIDINameDocument PUBLIC \"-//MIDI Manufacturers Association//DTD MIDINameDocument 1.0//EN\" \"http://www.midi.org/dtds/MIDINameDocument10.dtd\">\n{}",
        root.to_xml()
    )
}

pub fn parse(text: &str) -> Result<Document, String> {
    let b = text.as_bytes();
    let (mut i, mut line) = (0usize, 1usize);
    let mut stack: Vec<Node> = Vec::new();
    let (mut doctype, mut root): (Option<String>, Option<Node>) = (None, None);
    while i < b.len() {
        if b[i] == b'<' {
            let rest = &text[i..];
            let end_of = |pat: &str| rest.find(pat).ok_or_else(|| format!("line {line}: unterminated {pat}"));
            if rest.starts_with("<!--") {
                let e = end_of("-->")?;
                line += rest[..e].matches('\n').count();
                i += e + 3;
            } else if rest.starts_with("<?") {
                let e = end_of("?>")?;
                line += rest[..e].matches('\n').count();
                i += e + 2;
            } else if rest.starts_with("<!DOCTYPE") {
                let e = end_of(">")?;
                doctype = Some(rest[9..e].trim().to_string());
                line += rest[..e].matches('\n').count();
                i += e + 1;
            } else if rest.starts_with("</") {
                let e = end_of(">")?;
                let name = rest[2..e].trim();
                let node = stack.pop().ok_or_else(|| format!("line {line}: closing tag {name} with nothing open"))?;
                if node.name != name {
                    return Err(format!("line {line}: closing tag {name} for {}", node.name));
                }
                i += e + 1;
                match stack.last_mut() {
                    Some(p) => p.children.push(node),
                    None if root.is_none() => root = Some(node),
                    None => return Err(format!("line {line}: a second root element")),
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
                    return Err(format!("line {line}: unterminated tag"));
                }
                let mut inner = &text[i + 1..j];
                let empty = inner.ends_with('/');
                if empty {
                    inner = &inner[..inner.len() - 1];
                }
                let name_end = inner.find(char::is_whitespace).unwrap_or(inner.len());
                let mut node = Node { name: inner[..name_end].to_string(), line, ..Node::default() };
                if node.name.is_empty() {
                    return Err(format!("line {line}: a tag with no name"));
                }
                let mut attrs = inner[name_end..].trim();
                while !attrs.is_empty() {
                    let eq = attrs.find('=').ok_or_else(|| format!("line {line}: an attribute with no value"))?;
                    let key = attrs[..eq].trim().to_string();
                    let after = attrs[eq + 1..].trim_start();
                    let q = after.chars().next().filter(|c| *c == '"' || *c == '\'').ok_or_else(|| format!("line {line}: an unquoted value"))?;
                    let close = after[1..].find(q).ok_or_else(|| format!("line {line}: an unterminated value"))?;
                    node.attrs.push((key, unescape(&after[1..1 + close])));
                    attrs = after[close + 2..].trim_start();
                }
                line += text[i..j].matches('\n').count();
                i = j + 1;
                if empty {
                    match stack.last_mut() {
                        Some(p) => p.children.push(node),
                        None if root.is_none() => root = Some(node),
                        None => return Err(format!("line {line}: a second root element")),
                    }
                } else {
                    stack.push(node);
                }
            }
        } else {
            let next = text[i..].find('<').map_or(b.len(), |n| i + n);
            let chunk = &text[i..next];
            line += chunk.matches('\n').count();
            if let Some(top) = stack.last_mut() {
                top.text.push_str(&unescape(chunk));
            } else if !chunk.trim().is_empty() {
                return Err(format!("line {line}: text outside the root element"));
            }
            i = next;
        }
    }
    if let Some(open) = stack.last() {
        return Err(format!("line {}: {} is never closed", open.line, open.name));
    }
    let mut root = root.ok_or("no root element")?;
    fn trim(n: &mut Node) {
        let t = n.text.trim().to_string();
        n.text = t;
        n.children.iter_mut().for_each(trim);
    }
    trim(&mut root);
    Ok(Document { doctype, root })
}
