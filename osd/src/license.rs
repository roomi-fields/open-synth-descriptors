//! The shape of a `license` of 0.7: an SPDX expression (SPDX 2.3, annex "SPDX License
//! Expressions"), read without the SPDX registry. The analysis keeps the expression in postfix
//! order and walks it with a stack of its own, so an expression nested however deep costs no call
//! stack, to read, to judge or to drop.

/// A well-formed SPDX licence expression: ids, `WITH` an exception, `AND`, `OR` and parentheses
/// (`spec/0.7.md` 6.7). The special values `public-domain`, `NOASSERTION` and `NONE` are no
/// expression: a `license` holds one of them alone.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LicenseExpression {
    /// The expression in postfix order: never empty, each operator after its two operands.
    items: Vec<Item>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Item {
    Leaf { id: String, exception: Option<String> },
    Operator(Operator),
}

/// The operators that combine two operands, the one that binds tighter last.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Operator {
    Or,
    And,
}

/// What waits, while reading, for the end of its right operand.
enum Pending {
    Parenthesis,
    Operator(Operator),
}

impl LicenseExpression {
    /// The analysis of `text`; `None` for a malformed expression, and for `public-domain`,
    /// `NOASSERTION` and `NONE`, whatever their case. Only the space separates, and none stands
    /// at either end.
    pub fn parse(text: &str) -> Option<LicenseExpression> {
        if text.starts_with(' ') || text.ends_with(' ') {
            return None;
        }
        let mut items = Vec::new();
        let mut pending = Vec::new();
        let mut tokens = tokens(text);
        // Whether an operand comes next, and whether the last token is an id that may take `WITH`.
        let mut operand = true;
        let mut with = false;
        while let Some(token) = tokens.next() {
            if operand {
                if token == "(" {
                    pending.push(Pending::Parenthesis);
                    continue;
                }
                if !is_simple(token) {
                    return None;
                }
                items.push(Item::Leaf { id: token.to_string(), exception: None });
                operand = false;
                with = true;
                continue;
            }
            match token {
                "WITH" if with => {
                    let word = tokens.next().filter(|w| is_idstring(w) && !is_reserved(w))?;
                    if let Some(Item::Leaf { exception, .. }) = items.last_mut() {
                        *exception = Some(word.to_string());
                    }
                }
                "AND" | "OR" => {
                    let operator = if token == "AND" { Operator::And } else { Operator::Or };
                    while let Some(Pending::Operator(before)) = pending.last() {
                        if *before < operator {
                            break;
                        }
                        items.push(Item::Operator(*before));
                        pending.pop();
                    }
                    pending.push(Pending::Operator(operator));
                    operand = true;
                }
                ")" => loop {
                    match pending.pop()? {
                        Pending::Parenthesis => break,
                        Pending::Operator(o) => items.push(Item::Operator(o)),
                    }
                },
                _ => return None,
            }
            with = false;
        }
        if operand {
            return None;
        }
        while let Some(last) = pending.pop() {
            match last {
                Pending::Parenthesis => return None,
                Pending::Operator(o) => items.push(Item::Operator(o)),
            }
        }
        Some(LicenseExpression { items })
    }

    /// The judgement of the expression: `leaf` of each id (its `+` included) and its exception,
    /// combined by `and` and `or` as SPDX binds them (`WITH`, then `AND`, then `OR`) and as the
    /// parentheses group them.
    pub fn judge<T>(&self, leaf: impl Fn(&str, Option<&str>) -> T, and: impl Fn(T, T) -> T, or: impl Fn(T, T) -> T) -> T {
        let mut judged = Vec::new();
        for item in &self.items {
            let next = match item {
                Item::Leaf { id, exception } => leaf(id, exception.as_deref()),
                Item::Operator(operator) => {
                    let (Some(right), Some(left)) = (judged.pop(), judged.pop()) else {
                        unreachable!("parse puts each operator after its two operands")
                    };
                    match operator {
                        Operator::And => and(left, right),
                        Operator::Or => or(left, right),
                    }
                }
            };
            judged.push(next);
        }
        let Some(whole) = judged.pop() else { unreachable!("parse gives at least one operand") };
        whole
    }
}

/// The tokens of `text`: each parenthesis, and each run of other characters than the space and the
/// parentheses.
fn tokens(text: &str) -> impl Iterator<Item = &str> {
    let mut rest = text;
    std::iter::from_fn(move || {
        rest = rest.trim_start_matches(' ');
        let length = match rest.find([' ', '(', ')']) {
            Some(0) => 1,
            Some(n) => n,
            None => rest.len(),
        };
        let (token, after) = rest.split_at(length);
        rest = after;
        Some(token).filter(|t| !t.is_empty())
    })
}

/// A word that is no id and no exception: an operator, or a special value whatever its case.
fn is_reserved(word: &str) -> bool {
    matches!(word, "AND" | "OR" | "WITH") || ["public-domain", "NOASSERTION", "NONE"].iter().any(|v| v.eq_ignore_ascii_case(word))
}

/// An idstring of SPDX: letters, digits, `.` and `-`.
fn is_idstring(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-'))
}

/// A licence by itself: an id (an idstring of SPDX, `+` admitted after its first character), or
/// `DocumentRef-<idstring>:LicenseRef-<idstring>`. Its case is free.
fn is_simple(word: &str) -> bool {
    if let Some((document, license)) = word.split_once(':') {
        return document.strip_prefix("DocumentRef-").is_some_and(is_idstring)
            && license.strip_prefix("LicenseRef-").is_some_and(is_idstring);
    }
    let mut chars = word.chars();
    !is_reserved(word)
        && chars.next().is_some_and(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-'))
        && chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '+' | '-'))
}
