//! CSV as RFC 4180 writes it: records of cells, quoted where a cell holds a comma, a quote
//! or a line break.

/// A record with the line of the file it starts on (1 for the first).
pub(crate) struct Record {
    pub line: usize,
    pub cells: Vec<String>,
}

/// Why a text is not CSV: the line where its last quoted cell opens.
pub(crate) struct Unterminated(pub usize);

/// The records of `text`; a line break (`\n` or `\r\n`) ends a record unless quoted, and a
/// blank line holds no record.
pub(crate) fn parse(text: &str) -> Result<Vec<Record>, Unterminated> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut records = Vec::new();
    let (mut cells, mut cell) = (Vec::new(), String::new());
    let (mut quoted, mut was_quoted) = (false, false);
    let (mut line, mut start, mut opened) = (1usize, 1usize, 1usize);
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\n' {
            line += 1;
        }
        match (c, quoted) {
            ('"', true) if chars.peek() == Some(&'"') => {
                cell.push('"');
                chars.next();
            }
            ('"', true) => quoted = false,
            ('"', false) if cell.is_empty() && !was_quoted => {
                quoted = true;
                was_quoted = true;
                opened = line;
            }
            (',', false) => {
                cells.push(std::mem::take(&mut cell));
                was_quoted = false;
            }
            ('\r', false) if chars.peek() == Some(&'\n') => {}
            ('\n', false) => {
                if !(cells.is_empty() && cell.is_empty() && !was_quoted) {
                    cells.push(std::mem::take(&mut cell));
                    records.push(Record { line: start, cells: std::mem::take(&mut cells) });
                }
                was_quoted = false;
                start = line;
            }
            _ => cell.push(c),
        }
    }
    if quoted {
        return Err(Unterminated(opened));
    }
    if !(cells.is_empty() && cell.is_empty() && !was_quoted) {
        cells.push(cell);
        records.push(Record { line: start, cells });
    }
    Ok(records)
}

/// One record as a line, each cell quoted when it needs it.
pub(crate) fn line(cells: &[&str]) -> String {
    let mut out = String::new();
    for (n, cell) in cells.iter().enumerate() {
        if n > 0 {
            out.push(',');
        }
        if cell.contains([',', '"', '\n', '\r']) {
            out.push('"');
            out.push_str(&cell.replace('"', "\"\""));
            out.push('"');
        } else {
            out.push_str(cell);
        }
    }
    out.push('\n');
    out
}
