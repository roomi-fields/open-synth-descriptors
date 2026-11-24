//! The program `osd-reference`: the words `build`, `count`, `undescribed`.

use std::path::Path;
use std::process::ExitCode;

fn usage() -> ExitCode {
    eprintln!("usage: osd-reference build <schemas folder> <output folder>");
    eprintln!("       osd-reference count <schemas folder>");
    eprintln!("       osd-reference undescribed <schemas folder>");
    ExitCode::from(2)
}

fn fail(message: String) -> ExitCode {
    eprintln!("osd-reference: {message}");
    ExitCode::from(1)
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (word, rest) = match args.split_first() {
        Some((w, r)) => (w.as_str(), r),
        None => return usage(),
    };
    let wanted = match word {
        "build" => 2,
        "count" | "undescribed" => 1,
        _ => return usage(),
    };
    if rest.len() != wanted {
        return usage();
    }
    let schemas = match osd_reference::load(Path::new(&rest[0])) {
        Ok(s) => s,
        Err(e) => return fail(e),
    };
    match word {
        "build" => {
            let out = Path::new(&rest[1]);
            if let Err(e) = std::fs::create_dir_all(out) {
                return fail(format!("{}: {e}", out.display()));
            }
            for (kind, schema) in &schemas {
                let file = out.join(format!("{kind}.md"));
                if let Err(e) = std::fs::write(&file, osd_reference::page(kind, schema)) {
                    return fail(format!("{}: {e}", file.display()));
                }
            }
        }
        "count" => {
            for (kind, schema) in &schemas {
                let (d, t) = osd_reference::count(schema);
                println!("{kind}: {d} of {t}");
            }
        }
        _ => {
            let mut lines: Vec<String> = schemas
                .iter()
                .flat_map(|(kind, schema)| {
                    osd_reference::properties(schema)
                        .into_iter()
                        .filter(|p| p.description.is_none())
                        .map(move |p| format!("{kind} {}", p.pointer))
                })
                .collect();
            lines.sort();
            for l in lines {
                println!("{l}");
            }
        }
    }
    ExitCode::SUCCESS
}
