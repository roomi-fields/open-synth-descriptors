//! The crate osd-cli: its interface, docs/INTERFACE.md, says what it offers.
//!
//! Each word reads the files it is given, calls one operation of `osd`, and
//! prints the result or the faults; no rule of the format lives here.

use std::fs::OpenOptions;
use std::io::Write;

const VALIDATE_SHAPE: &str = "osd validate [--kind <kind>] <file>…";
const CONVERT_SHAPE: &str = "osd convert [--kind <kind>] [--to <version>] <file> [-o <file>] [--force]";
const INIT_SHAPE: &str = "osd init <kind> [-o <file>] [--force]";
const IMPORT_SHAPE: &str =
    "osd import <format> <file> [--source-url <url>] [--source-author <name>] --license <spdx> [--into <descriptor>] [-o <file>] [--force]";
const EXPORT_SHAPE: &str = "osd export <format> <file> [-o <file>] [--force]";

/// The kinds, with the template embedded for each.
const KINDS: [(&str, osd::Kind, &str); 5] = [
    ("device", osd::Kind::Device, include_str!("../templates/device.json")),
    ("profile", osd::Kind::Profile, include_str!("../templates/profile.json")),
    ("sound", osd::Kind::Sound, include_str!("../templates/sound.json")),
    ("fingerprint", osd::Kind::Fingerprint, include_str!("../templates/fingerprint.json")),
    ("firmware", osd::Kind::Firmware, include_str!("../templates/firmware.json")),
];

fn kind_names() -> String {
    KINDS.iter().map(|k| k.0).collect::<Vec<_>>().join(", ")
}

fn format_names() -> String {
    osd_formats::Format::ALL.iter().map(|f| f.name()).collect::<Vec<_>>().join(", ")
}

fn general_help() -> String {
    format!(
        "osd: check, convert and start an Open Synth Descriptors file.\n\n\
         Usage:\n  {VALIDATE_SHAPE}\n  {CONVERT_SHAPE}\n  {INIT_SHAPE}\n  {IMPORT_SHAPE}\n  {EXPORT_SHAPE}\n\n\
         Words:\n  validate  say whether each file is well written\n  \
         convert   write a descriptor in the latest version\n  \
         {}   write a template to fill\n  \
         import    write the descriptor of a file in another format ({})\n  \
         export    write a descriptor in another format ({})\n\n\
         Options:\n  --help     show this help (also after a word)\n  --version  show the versions of the format read\n",
        word_init(),
        format_names(),
        format_names()
    )
}

/// The word that writes a template.
fn word_init() -> &'static str {
    "init"
}

fn word_help(word: &str) -> String {
    match word {
        "validate" => format!(
            "{VALIDATE_SHAPE}\n\nPrints `<file>: ok` for a file without fault, and on the error output one line\n\
             per fault: `<file>: <Kind> at \"<pointer>\": <rule>`.\n\
             The kind is read from the `$schema` of the file; `--kind <kind>` ({}) gives it\n\
             for a file without `$schema`.\nExit 0 when every file is right, 1 when one has a fault, 2 when misused.\n",
            kind_names()
        ),
        "convert" => format!(
            "{CONVERT_SHAPE}\n\nWrites the descriptor in the latest published version on the standard output, or in the\n\
             file `-o` names; an existing file is replaced only with `--force`.\n\
             `--to <version>` names the version to write: the 0.6 draft only when asked for by name.\n\
             The kind is read from the `$schema` of the file; `--kind <kind>` ({}) gives it\n\
             for a file without `$schema`.\nExit 1 when the text cannot be converted, 2 when misused.\n",
            kind_names()
        ),
        "import" => format!(
            "{IMPORT_SHAPE}\n\nWrites the descriptor of the file, a file of the format ({}), on the standard output, or in\n\
             the file `-o` names; an existing file is replaced only with `--force`.\n\
             The descriptor carries the source given by `--source-url`, `--source-author` and `--license`:\n\
             one of the url or the author, and the license, are needed (exit 2 without them).\n\
             `--into <descriptor>` (for midi-csv and midi-triggers-csv) adds the entries of the file to that\n\
             descriptor, which keeps its identity and its other blocks, and its sources then name both files.\n\
             What the file could not say is printed on the error output, one line per loss.\n\
             Exit 1 when the file is not valid in its format, 2 when misused.\n",
            format_names()
        ),
        "export" => format!(
            "{EXPORT_SHAPE}\n\nWrites the descriptor file, read alone, in the format ({}) on the standard output, or in\n\
             the file `-o` names; an existing file is replaced only with `--force`.\n\
             What the format cannot carry is printed on the error output, one line per loss.\n\
             Exit 1 when the descriptor cannot be read, resolved or written, 2 when misused.\n",
            format_names()
        ),
        _ => format!(
            "{INIT_SHAPE}\n\nWrites a template of the kind ({}) on the standard output, or in the file\n\
             `-o` names; an existing file is replaced only with `--force`.\n\
             Each place to fill is a string that starts with `FILL: `.\n",
            kind_names()
        ),
    }
}

/// A misuse: the message, then the help, on the error output; exit 2.
fn misuse(message: &str, help: &str) -> i32 {
    eprintln!("osd: {message}\n\n{help}");
    2
}

#[derive(Default)]
struct Options {
    kind: Option<String>,
    to: Option<String>,
    output: Option<String>,
    force: bool,
    source_url: Option<String>,
    source_author: Option<String>,
    license: Option<String>,
    into: Option<String>,
    positional: Vec<String>,
}

fn parse(args: &[String], takes_kind: bool, takes_output: bool, takes_source: bool, takes_into: bool) -> Result<Options, String> {
    let mut o = Options::default();
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--kind" if takes_kind => o.kind = Some(it.next().ok_or("`--kind` needs a kind")?.clone()),
            "--to" if takes_kind && takes_output => o.to = Some(it.next().ok_or("`--to` needs a version")?.clone()),
            "-o" if takes_output => o.output = Some(it.next().ok_or("`-o` needs a file")?.clone()),
            "--force" if takes_output => o.force = true,
            "--source-url" if takes_source => o.source_url = Some(it.next().ok_or("`--source-url` needs a url")?.clone()),
            "--source-author" if takes_source => o.source_author = Some(it.next().ok_or("`--source-author` needs a name")?.clone()),
            "--into" if takes_into => o.into = Some(it.next().ok_or("`--into` needs a descriptor")?.clone()),
            "--license" if takes_source => o.license = Some(it.next().ok_or("`--license` needs an SPDX id")?.clone()),
            s if s.starts_with('-') && s.len() > 1 => return Err(format!("unknown option `{s}`")),
            _ => o.positional.push(a.clone()),
        }
    }
    Ok(o)
}

fn kind_named(name: &str) -> Option<osd::Kind> {
    KINDS.iter().find(|k| k.0 == name).map(|k| k.1)
}

/// The kind of a file: the one given with `--kind`, else the one its `$schema` declares.
fn kind_of(given: Option<osd::Kind>, text: &str) -> Result<osd::Kind, String> {
    given
        .or_else(|| osd::declared_kind(text))
        .ok_or_else(|| format!("the file declares no kind in `$schema`: give `--kind <kind>` ({})", kind_names()))
}

fn fault_line(file: &str, f: &osd::Fault) -> String {
    format!("{file}: {:?} at \"{}\": {}", f.kind, f.pointer, f.rule)
}

/// Writes `text` to `path`, refusing an existing file unless `force`.
fn write_file(path: &str, text: &str, force: bool) -> Result<(), String> {
    let mut opts = OpenOptions::new();
    opts.write(true);
    if force {
        opts.create(true).truncate(true);
    } else {
        opts.create_new(true);
    }
    let mut f = opts.open(path).map_err(|e| {
        if e.kind() == std::io::ErrorKind::AlreadyExists {
            format!("{path} exists: add `--force` to replace it")
        } else {
            format!("{path}: {e}")
        }
    })?;
    f.write_all(text.as_bytes()).map_err(|e| format!("{path}: {e}"))
}

/// Gives `text` to the standard output or to the file `-o` names.
fn emit(text: &str, o: &Options) -> i32 {
    match &o.output {
        None => {
            print!("{text}");
            0
        }
        Some(path) => match write_file(path, text, o.force) {
            Ok(()) => 0,
            Err(m) => {
                eprintln!("osd: {m}");
                2
            }
        },
    }
}

fn given_kind(o: &Options, help: &str) -> Result<Option<osd::Kind>, i32> {
    match &o.kind {
        None => Ok(None),
        Some(n) => kind_named(n).map(Some).ok_or_else(|| misuse(&format!("`--kind {n}`: not a kind; the kinds are {}", kind_names()), help)),
    }
}

fn validate(o: Options, help: &str) -> i32 {
    if o.positional.is_empty() {
        return misuse("validate needs a file", help);
    }
    let given = match given_kind(&o, help) {
        Ok(k) => k,
        Err(c) => return c,
    };
    let (mut faulty, mut misused) = (false, false);
    for file in &o.positional {
        let text = match std::fs::read_to_string(file) {
            Ok(t) => t,
            Err(e) => {
                eprintln!("osd: {file}: {e}");
                misused = true;
                continue;
            }
        };
        let kind = match kind_of(given, &text) {
            Ok(k) => k,
            Err(m) => {
                eprintln!("osd: {file}: {m}");
                misused = true;
                continue;
            }
        };
        match osd::read(&text, kind) {
            Ok(_) => println!("{file}: ok"),
            Err(f) => {
                eprintln!("{}", fault_line(file, &f));
                faulty = true;
            }
        }
    }
    if misused {
        2
    } else {
        faulty as i32
    }
}

fn convert(o: Options, help: &str) -> i32 {
    if o.positional.len() != 1 {
        return misuse("convert needs one file", help);
    }
    let given = match given_kind(&o, help) {
        Ok(k) => k,
        Err(c) => return c,
    };
    let target = match &o.to {
        None => osd::PUBLISHED,
        Some(name) => match osd::VERSIONS.iter().copied().find(|v| v.name() == name) {
            Some(v) => v,
            None => {
                let known: Vec<&str> = osd::VERSIONS.iter().map(|v| v.name()).collect();
                return misuse(&format!("`{name}` is not a version read; the versions are {}", known.join(", ")), help);
            }
        },
    };
    let file = &o.positional[0];
    let text = match std::fs::read_to_string(file) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("osd: {file}: {e}");
            return 2;
        }
    };
    let kind = match kind_of(given, &text) {
        Ok(k) => k,
        Err(m) => {
            eprintln!("osd: {file}: {m}");
            return 2;
        }
    };
    match osd::convert_to(&text, kind, target) {
        Ok(out) => emit(&out, &o),
        Err(f) => {
            eprintln!("{}", fault_line(file, &f));
            1
        }
    }
}

/// Prints each loss on the error output, one per line.
fn print_losses(lost: &[osd_formats::Loss]) {
    for l in lost {
        eprintln!("{}: {}", l.place, l.reason);
    }
}

fn format_named(name: &str, help: &str) -> Result<osd_formats::Format, i32> {
    osd_formats::Format::from_name(name)
        .ok_or_else(|| misuse(&format!("`{name}` is not a format; the formats are {}", format_names()), help))
}

fn read_input(file: &str) -> Result<String, i32> {
    std::fs::read_to_string(file).map_err(|e| {
        eprintln!("osd: {file}: {e}");
        2
    })
}

fn import(o: Options, help: &str) -> i32 {
    let [name, file] = o.positional.as_slice() else {
        return misuse("import needs a format and a file", help);
    };
    let format = match format_named(name, help) {
        Ok(f) => f,
        Err(c) => return c,
    };
    let text = match read_input(file) {
        Ok(t) => t,
        Err(c) => return c,
    };
    let source = osd::Source {
        kind: "community-re".to_string(),
        url: o.source_url.clone(),
        firmware: None,
        author: o.source_author.clone(),
        attribution: None,
        license: o.license.clone(),
        commit: None,
        imported_at: None,
        notes: None,
        extra: Default::default(),
    };
    let translated = match &o.into {
        None => osd_formats::import(&text, format, &source),
        Some(base_file) => {
            let base_text = match read_input(base_file) {
                Ok(t) => t,
                Err(c) => return c,
            };
            match osd::read(&base_text, osd::Kind::Device) {
                Ok(osd::Descriptor::Device(base)) => osd_formats::import_into(*base, &text, format, &source),
                Ok(_) => {
                    eprintln!("{base_file}: not a device descriptor");
                    return 1;
                }
                Err(f) => {
                    eprintln!("{}", fault_line(base_file, &f));
                    return 1;
                }
            }
        }
    };
    match translated {
        Ok(t) => {
            print_losses(&t.lost);
            emit(&osd::write(&osd::Descriptor::Device(Box::new(t.value))), &o)
        }
        Err(f) => {
            eprintln!("{file}: {f}");
            if f.kind == osd_formats::FaultKind::SourceIncomplete {
                2
            } else {
                1
            }
        }
    }
}

/// The one file of an export: the catalogue holds it alone, so a link no file answers is a fault.
struct Alone {
    id: String,
    text: String,
}

impl Alone {
    /// The file under an id of the maker its `midi_of` names (a link stays within one maker).
    fn new(text: String) -> Alone {
        let maker = match osd::read(&text, osd::Kind::Device) {
            Ok(osd::Descriptor::Device(d)) => d.midi_of.as_deref().and_then(|t| t.split('/').next()).map(str::to_string),
            _ => None,
        };
        Alone { id: format!("{}/file", maker.unwrap_or_else(|| "file".to_string())), text }
    }
}

impl osd::Files for Alone {
    fn get(&self, id: &str) -> Option<String> {
        (id == self.id).then(|| self.text.clone())
    }
}

fn export(o: Options, help: &str) -> i32 {
    let [name, file] = o.positional.as_slice() else {
        return misuse("export needs a format and a file", help);
    };
    let format = match format_named(name, help) {
        Ok(f) => f,
        Err(c) => return c,
    };
    let text = match read_input(file) {
        Ok(t) => t,
        Err(c) => return c,
    };
    let files = Alone::new(text);
    let resolved = match osd::resolve(&files.id, &files) {
        Ok(r) => r,
        Err(f) => {
            eprintln!("{}", fault_line(file, &f));
            return 1;
        }
    };
    match osd_formats::export(&resolved, format) {
        Ok(t) => {
            print_losses(&t.lost);
            emit(&t.value, &o)
        }
        Err(f) => {
            eprintln!("{file}: {f}");
            1
        }
    }
}

fn template(o: Options, help: &str) -> i32 {
    let [name] = o.positional.as_slice() else {
        return misuse(&format!("{} needs one kind: {}", word_init(), kind_names()), help);
    };
    match KINDS.iter().find(|k| k.0 == name) {
        Some(k) => emit(k.2, &o),
        None => misuse(&format!("`{name}` is not a kind; the kinds are {}", kind_names()), help),
    }
}

/// Runs the program on its arguments (without its own name) and returns its exit code.
pub fn run(args: Vec<String>) -> i32 {
    let help = general_help();
    let Some(word) = args.first() else {
        return misuse("a word is needed", &help);
    };
    match word.as_str() {
        "--help" | "-h" => {
            print!("{help}");
            0
        }
        "--version" | "-V" => {
            let versions: Vec<&str> = osd::VERSIONS.iter().map(|v| v.name()).collect();
            println!("osd {} (reads the format versions {})", env!("CARGO_PKG_VERSION"), versions.join(", "));
            0
        }
        w @ ("validate" | "convert" | "import" | "export") | w @ "init" => {
            let rest = &args[1..];
            if rest.iter().any(|a| a == "--help" || a == "-h") {
                print!("{}", word_help(w));
                return 0;
            }
            let wh = word_help(w);
            let parsed = match w {
                "validate" => parse(rest, true, false, false, false),
                "convert" => parse(rest, true, true, false, false),
                "import" => parse(rest, false, true, true, true),
                _ => parse(rest, false, true, false, false),
            };
            match parsed {
                Err(m) => misuse(&m, &help),
                Ok(o) => match w {
                    "validate" => validate(o, &wh),
                    "convert" => convert(o, &wh),
                    "import" => import(o, &wh),
                    "export" => export(o, &wh),
                    _ => template(o, &wh),
                },
            }
        }
        other => misuse(&format!("`{other}` is not a word"), &help),
    }
}
