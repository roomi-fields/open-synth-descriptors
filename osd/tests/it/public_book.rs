//! The book of the specification: the pinned tool and its installer, the build that assembles the
//! book from spec/ and the generated reference, the link check, and the workflows that call them.
//! Medium: the scripts run as processes on fixtures and on the real spec/.

use crate::common;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;

const PIN: &str = "tools/mdbook.pin";
const INSTALL: &str = "tools/install_mdbook.py";
const BUILD_BOOK: &str = "tools/build_book.py";
const CHECK_BOOK: &str = "tools/check_book.py";
const BUILD_SITE: &str = "tools/build_site.py";
const PAGES: &str = ".github/workflows/pages.yml";
const CHECK: &str = ".github/workflows/check.yml";
const REPO: &str = "https://github.com/roomi-fields/open-synth-descriptors";

struct Run {
    code: Option<i32>,
    out: String,
    stdout: String,
}

fn python(script: &str, args: &[&str]) -> Run {
    python_with(script, args, &[])
}

/// Runs the script with each named variable set to its value, or removed when it has none.
fn python_with(script: &str, args: &[&str], vars: &[(&str, Option<&str>)]) -> Run {
    let script = common::repo().join(script);
    let mut last = None;
    for interpreter in ["python3", "python"] {
        let mut command = Command::new(interpreter);
        command.arg(&script).args(args);
        for (name, value) in vars {
            match value {
                Some(value) => command.env(name, value),
                None => command.env_remove(name),
            };
        }
        match command.output() {
            Ok(o) => {
                let stdout = String::from_utf8_lossy(&o.stdout).to_string();
                return Run {
                    code: o.status.code(),
                    out: format!("{stdout}{}", String::from_utf8_lossy(&o.stderr)),
                    stdout,
                };
            }
            Err(e) => last = Some(e),
        }
    }
    panic!("no python interpreter: {}", last.unwrap());
}

fn read(rel: &str) -> String {
    std::fs::read_to_string(common::repo().join(rel)).unwrap_or_else(|e| panic!("{rel}: {e}"))
}

fn write(root: &Path, rel: &str, text: &str) {
    let path = root.join(rel);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}

fn files_under(root: &Path) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    let mut todo = vec![root.to_path_buf()];
    while let Some(dir) = todo.pop() {
        for entry in std::fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                todo.push(path);
            } else {
                out.insert(path.strip_prefix(root).unwrap().to_string_lossy().replace('\\', "/"));
            }
        }
    }
    out
}

// ---- The pin and the installer ----

fn sha256_of(file: &Path) -> String {
    let o = Command::new("sha256sum").arg(file).output().expect("sha256sum");
    String::from_utf8_lossy(&o.stdout).split_whitespace().next().unwrap().to_string()
}

/// A fake release archive holding one executable `mdbook`, in a fresh cache folder, and a pin
/// that names it by its release name at an address that cannot be fetched.
struct Fixture {
    _dir: TempDir,
    cache: PathBuf,
    pin: PathBuf,
    archive: PathBuf,
}

fn fixture(pinned_sha: Option<&str>) -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let cache = dir.path().join("cache");
    std::fs::create_dir_all(&cache).unwrap();
    let src = dir.path().join("src");
    write(&src, "mdbook", "#!/bin/sh\necho fixture-mdbook\n");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(src.join("mdbook"), std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    let name = "mdbook-v9.9.9-x86_64-unknown-linux-gnu.tar.gz";
    let archive = cache.join(name);
    let status = Command::new("tar").arg("czf").arg(&archive).arg("-C").arg(&src).arg("mdbook").status().unwrap();
    assert!(status.success());
    let sha = pinned_sha.map(String::from).unwrap_or_else(|| sha256_of(&archive));
    let pin = dir.path().join("mdbook.pin");
    std::fs::write(
        &pin,
        format!("version = 9.9.9\nurl = http://127.0.0.1:1/releases/v9.9.9/{name}\nsha256 = {sha}\n"),
    )
    .unwrap();
    Fixture { _dir: dir, cache, pin, archive }
}

fn install(f: &Fixture) -> Run {
    python(INSTALL, &["--pin", f.pin.to_str().unwrap(), "--cache", f.cache.to_str().unwrap()])
}

#[test]
fn the_pin_holds_the_version_the_address_and_the_checksum_of_the_release_archive() {
    let pin = read(PIN);
    assert!(pin.contains("0.5.4"), "{pin}");
    assert!(
        pin.contains("https://github.com/rust-lang/mdBook/releases/download/v0.5.4/mdbook-v0.5.4-x86_64-unknown-linux-gnu.tar.gz"),
        "{pin}"
    );
    assert!(pin.contains("3f28de05dafca9d0f2eab99c662116b0e37b89b1d96a08f8f430b9eeae958cd7"), "{pin}");
}

#[test]
fn the_version_and_the_checksum_are_written_once_in_the_pin() {
    for rel in [INSTALL, BUILD_BOOK, CHECK_BOOK, PAGES, CHECK] {
        let text = read(rel);
        for literal in ["0.5.4", "3f28de05dafca9d0f2eab99c662116b0e37b89b1d96a08f8f430b9eeae958cd7", "mdbook-v0"] {
            assert!(!text.contains(literal), "{rel} holds the literal {literal}: it lives in {PIN}");
        }
    }
}

#[test]
fn an_archive_in_the_cache_that_matches_the_pin_is_used_without_any_download() {
    let f = fixture(None);
    let run = install(&f);
    assert_eq!(run.code, Some(0), "{}", run.out);
    let binary = PathBuf::from(run.stdout.lines().last().expect("the binary path").trim());
    assert!(binary.is_file(), "{binary:?} is not a file");
    assert_eq!(binary.file_name().unwrap(), "mdbook");
    let ran = Command::new(&binary).output().expect("the installed binary runs");
    assert_eq!(String::from_utf8_lossy(&ran.stdout).trim(), "fixture-mdbook");
}

#[test]
fn an_archive_whose_checksum_differs_from_the_pin_is_refused_and_kept_as_found() {
    let f = fixture(Some(&"0".repeat(64)));
    let before = std::fs::read(&f.archive).unwrap();
    let run = install(&f);
    assert_ne!(run.code, Some(0), "{}", run.out);
    assert!(run.out.to_lowercase().contains("checksum") || run.out.contains("sha256"), "{}", run.out);
    assert!(run.out.contains(&"0".repeat(64)), "the pinned checksum is named: {}", run.out);
    assert_eq!(std::fs::read(&f.archive).unwrap(), before, "the archive is replaced silently");
}

#[test]
fn a_tool_that_cannot_be_installed_fails_and_is_never_skipped() {
    let f = fixture(None);
    std::fs::remove_file(&f.archive).unwrap();
    let run = install(&f);
    assert_ne!(run.code, Some(0), "{}", run.out);
    assert!(!run.stdout.lines().any(|l| l.trim().ends_with("mdbook")), "a binary path is printed: {}", run.stdout);
}

#[test]
fn the_book_build_fails_when_its_tool_cannot_be_installed_and_writes_no_book() {
    let f = fixture(None);
    std::fs::remove_file(&f.archive).unwrap();
    let out = tempfile::tempdir().unwrap();
    let book = out.path().join("book");
    let run = python(
        BUILD_BOOK,
        &[book.to_str().unwrap(), "--pin", f.pin.to_str().unwrap(), "--cache", f.cache.to_str().unwrap()],
    );
    assert_ne!(run.code, Some(0), "{}", run.out);
    assert!(!book.join("index.html").exists(), "a book is written without its tool");
}

// ---- The folder of the tool, named by MDBOOK ----

const FOLDER: &str = "MDBOOK";

/// A fake book tool: `mdbook build <work> -d <out>` writes one page into `<out>`.
const FAKE_MDBOOK: &str = "#!/bin/sh\nmkdir -p \"$4\" && echo fixture-book > \"$4/index.html\"\n";

/// A fake reference generator: `build <schemas> <out>` writes one page into `<out>`.
const FAKE_REFERENCE: &str = "#!/bin/sh\nmkdir -p \"$3\" && echo '# Device' > \"$3/device.md\"\n";

fn executable(path: &Path, text: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
}

fn text(path: &Path) -> &str {
    path.to_str().unwrap()
}

/// A release archive of the fake book tool at an address that is a local file or cannot be fetched,
/// a folder for the tool, and homes of the user.
struct Tool {
    dir: TempDir,
    pin: PathBuf,
    release: PathBuf,
}

const ARCHIVE: &str = "mdbook-v9.9.9-x86_64-unknown-linux-gnu.tar.gz";

impl Tool {
    fn new() -> Tool {
        Tool::holding(FAKE_MDBOOK)
    }

    /// The same, the book tool being the script given.
    fn holding(mdbook: &str) -> Tool {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("src");
        executable(&src.join("mdbook"), mdbook);
        let release = dir.path().join("releases").join(ARCHIVE);
        std::fs::create_dir_all(release.parent().unwrap()).unwrap();
        let status = Command::new("tar").arg("czf").arg(&release).arg("-C").arg(&src).arg("mdbook").status().unwrap();
        assert!(status.success());
        let pin = dir.path().join("mdbook.pin");
        let tool = Tool { dir, pin, release };
        tool.pinned_at(&format!("file://{}", tool.release.display()));
        tool
    }

    fn pinned_at(&self, url: &str) {
        let sha = sha256_of(&self.release);
        std::fs::write(&self.pin, format!("version = 9.9.9\nurl = {url}\nsha256 = {sha}\n")).unwrap();
    }

    /// The pin now names an address that cannot be fetched.
    fn offline(&self) {
        self.pinned_at(&format!("http://127.0.0.1:1/releases/v9.9.9/{ARCHIVE}"));
    }

    fn path(&self, rel: &str) -> PathBuf {
        self.dir.path().join(rel)
    }

    /// Fills the folder as a successful install leaves it: the pinned archive and its binary.
    fn fill(&self, folder: &Path) {
        let run = python(INSTALL, &["--pin", text(&self.pin), "--cache", text(folder)]);
        assert_eq!(run.code, Some(0), "{}", run.out);
    }

    /// A home where the default folder of the tool cannot be created: `.cache` is a file.
    fn sealed_home(&self) -> PathBuf {
        let home = self.path("sealed-home");
        write(&home, ".cache", "not a folder\n");
        home
    }

    /// A writable home, and the default folder of the tool under it.
    fn open_home(&self) -> (PathBuf, PathBuf) {
        let home = self.path("open-home");
        std::fs::create_dir_all(&home).unwrap();
        // The name of the shared cache folder is written in two parts, as in the installer.
        let default = home.join(".cache").join(concat!("grill", "house")).join("mdbook");
        (home, default)
    }

    fn install(&self, home: &Path, folder: Option<&str>, more: &[&str]) -> Run {
        let mut args = vec!["--pin", text(&self.pin)];
        args.extend_from_slice(more);
        python_with(INSTALL, &args, &[("HOME", Some(text(home))), (FOLDER, folder)])
    }

    fn build(&self, book: &Path, home: &Path, folder: Option<&str>, more: &[&str]) -> Run {
        let reference = self.path("fake-reference");
        if !reference.exists() {
            executable(&reference, FAKE_REFERENCE);
        }
        let mut args = vec![text(book), "--pin", text(&self.pin)];
        args.extend_from_slice(more);
        let vars = [("HOME", Some(text(home))), (FOLDER, folder), ("OSD_REFERENCE", Some(text(&reference)))];
        python_with(BUILD_BOOK, &args, &vars)
    }
}

/// A folder and everything under it without any write permission, given back when dropped.
struct ReadOnly(Vec<(PathBuf, std::fs::Permissions)>);

impl ReadOnly {
    fn seal(root: &Path) -> ReadOnly {
        let mut paths = vec![root.to_path_buf()];
        let mut next = 0;
        while next < paths.len() {
            if paths[next].is_dir() {
                for entry in std::fs::read_dir(&paths[next]).unwrap() {
                    paths.push(entry.unwrap().path());
                }
            }
            next += 1;
        }
        let mut sealed = ReadOnly(Vec::new());
        for path in paths.into_iter().rev() {
            let old = std::fs::metadata(&path).unwrap().permissions();
            let mut new = old.clone();
            new.set_readonly(true);
            std::fs::set_permissions(&path, new).unwrap();
            sealed.0.push((path, old));
        }
        sealed
    }
}

impl Drop for ReadOnly {
    fn drop(&mut self) {
        for (path, old) in self.0.iter().rev() {
            let _ = std::fs::set_permissions(path, old.clone());
        }
    }
}

/// Every entry under a folder, with the bytes of each file and the time of its last change.
fn snapshot(root: &Path) -> Vec<(PathBuf, Option<Vec<u8>>, std::time::SystemTime)> {
    let mut paths = vec![root.to_path_buf()];
    let mut out = Vec::new();
    while let Some(path) = paths.pop() {
        let meta = std::fs::metadata(&path).unwrap();
        let bytes = if meta.is_dir() {
            paths.extend(std::fs::read_dir(&path).unwrap().map(|e| e.unwrap().path()));
            None
        } else {
            Some(std::fs::read(&path).unwrap())
        };
        out.push((path, bytes, meta.modified().unwrap()));
    }
    out.sort();
    out
}

/// The path the installer printed last: a binary under the folder, which runs.
fn assert_installed_under(run: &Run, folder: &Path) {
    assert_eq!(run.code, Some(0), "{}", run.out);
    let binary = PathBuf::from(run.stdout.lines().last().expect("the binary path").trim());
    assert!(binary.starts_with(folder), "{binary:?} is not under {folder:?}");
    assert!(binary.is_file(), "{binary:?} is not a file");
    assert_eq!(binary.file_name().unwrap(), "mdbook");
}

#[test]
fn a_filled_read_only_folder_named_by_mdbook_serves_the_install_and_gains_nothing() {
    let tool = Tool::new();
    let folder = tool.path("shared-tools");
    tool.fill(&folder);
    tool.offline();
    let _sealed = ReadOnly::seal(&folder);
    let before = snapshot(&folder);
    let run = tool.install(&tool.sealed_home(), Some(text(&folder)), &[]);
    assert_installed_under(&run, &folder);
    assert_eq!(snapshot(&folder), before, "the install wrote into the folder");
}

#[test]
fn a_filled_read_only_folder_named_by_mdbook_serves_the_book_build_and_gains_nothing() {
    let tool = Tool::new();
    let folder = tool.path("shared-tools");
    tool.fill(&folder);
    tool.offline();
    let _sealed = ReadOnly::seal(&folder);
    let before = snapshot(&folder);
    let book = tool.path("book");
    let run = tool.build(&book, &tool.sealed_home(), Some(text(&folder)), &[]);
    assert_eq!(run.code, Some(0), "{}", run.out);
    assert!(book.join("index.html").is_file(), "no book written: {}", run.out);
    assert_eq!(snapshot(&folder), before, "the book build wrote into the folder");
}

/// The install and the book build refuse the read-only folder, naming it and the command that fills
/// it, and leave it as found.
fn assert_read_only_folder_refused(tool: &Tool, folder: &Path) {
    let _sealed = ReadOnly::seal(folder);
    let before = snapshot(folder);
    let home = tool.sealed_home();

    let run = tool.install(&home, Some(text(folder)), &[]);
    assert_eq!(run.code, Some(1), "{}", run.out);
    assert!(!run.out.contains("Traceback"), "{}", run.out);
    assert!(run.out.contains(text(folder)), "the folder is not named: {}", run.out);
    assert!(run.out.contains(INSTALL), "the command that fills the folder is not named: {}", run.out);
    assert!(!run.stdout.lines().any(|l| l.trim().ends_with("mdbook")), "a binary path is printed: {}", run.stdout);

    let book = tool.path("book");
    let run = tool.build(&book, &home, Some(text(folder)), &[]);
    assert_eq!(run.code, Some(1), "{}", run.out);
    assert!(!run.out.contains("Traceback"), "{}", run.out);
    assert!(run.out.contains(text(folder)) && run.out.contains(INSTALL), "{}", run.out);
    assert!(!book.exists(), "a book is written without its tool");

    assert_eq!(snapshot(folder), before, "the folder changed");
}

#[test]
fn an_empty_read_only_folder_named_by_mdbook_is_refused_with_the_command_that_fills_it() {
    let tool = Tool::new();
    let folder = tool.path("shared-tools");
    std::fs::create_dir_all(&folder).unwrap();
    assert_read_only_folder_refused(&tool, &folder);
}

#[test]
fn a_read_only_folder_holding_only_another_version_is_refused_with_the_command_that_fills_it() {
    let tool = Tool::new();
    let folder = tool.path("shared-tools");
    std::fs::create_dir_all(&folder).unwrap();
    std::fs::copy(&tool.release, folder.join("mdbook-v1.0.0-x86_64-unknown-linux-gnu.tar.gz")).unwrap();
    assert_read_only_folder_refused(&tool, &folder);
}

#[test]
fn a_read_only_folder_whose_archive_is_not_extracted_is_refused_with_the_command_that_fills_it() {
    let tool = Tool::new();
    let folder = tool.path("shared-tools");
    std::fs::create_dir_all(&folder).unwrap();
    std::fs::copy(&tool.release, folder.join(ARCHIVE)).unwrap();
    assert_read_only_folder_refused(&tool, &folder);
}

#[test]
fn a_read_only_folder_whose_extraction_lacks_the_binary_is_refused_with_the_command_that_fills_it() {
    let tool = Tool::new();
    let folder = tool.path("shared-tools");
    tool.fill(&folder);
    // What an interrupted extraction leaves: the folder of the pinned version, without its binary.
    for entry in std::fs::read_dir(&folder).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            std::fs::remove_file(path.join("mdbook")).unwrap();
        }
    }
    assert_read_only_folder_refused(&tool, &folder);
}

#[test]
fn an_empty_writable_folder_named_by_mdbook_receives_the_pinned_tool() {
    let tool = Tool::new();
    let folder = tool.path("shared-tools");
    std::fs::create_dir_all(&folder).unwrap();
    let (home, default) = tool.open_home();
    let run = tool.install(&home, Some(text(&folder)), &[]);
    assert_installed_under(&run, &folder);
    assert!(folder.join(ARCHIVE).is_file(), "the archive is not kept in the folder");
    assert!(!default.exists(), "the default folder is used although MDBOOK names another");
}

#[test]
fn a_writable_folder_named_by_mdbook_that_holds_another_version_receives_the_pinned_tool() {
    let tool = Tool::new();
    let folder = tool.path("shared-tools");
    write(&folder, "mdbook-v1.0.0-x86_64-unknown-linux-gnu.tar.gz", "an older archive");
    let (home, default) = tool.open_home();
    let run = tool.install(&home, Some(text(&folder)), &[]);
    assert_installed_under(&run, &folder);
    assert!(folder.join(ARCHIVE).is_file(), "the archive is not kept in the folder");
    assert!(!default.exists(), "the default folder is used although MDBOOK names another");
}

#[test]
fn an_explicit_cache_wins_over_mdbook_for_the_install() {
    let tool = Tool::new();
    let named = tool.path("shared-tools");
    let explicit = tool.path("explicit");
    std::fs::create_dir_all(&named).unwrap();
    let run = tool.install(&tool.sealed_home(), Some(text(&named)), &["--cache", text(&explicit)]);
    assert_installed_under(&run, &explicit);
    assert_eq!(std::fs::read_dir(&named).unwrap().count(), 0, "the folder of MDBOOK is written");
}

#[test]
fn an_explicit_cache_wins_over_mdbook_for_the_book_build() {
    let tool = Tool::new();
    let named = tool.path("shared-tools");
    let explicit = tool.path("explicit");
    std::fs::create_dir_all(&named).unwrap();
    let book = tool.path("book");
    let run = tool.build(&book, &tool.sealed_home(), Some(text(&named)), &["--cache", text(&explicit)]);
    assert_eq!(run.code, Some(0), "{}", run.out);
    assert!(book.join("index.html").is_file(), "no book written: {}", run.out);
    assert!(explicit.join(ARCHIVE).is_file(), "the explicit folder is not used");
    assert_eq!(std::fs::read_dir(&named).unwrap().count(), 0, "the folder of MDBOOK is written");
}

#[test]
fn without_mdbook_the_install_downloads_into_the_default_folder_under_the_home() {
    let tool = Tool::new();
    let (home, default) = tool.open_home();
    let run = tool.install(&home, None, &[]);
    assert_installed_under(&run, &default);
    assert!(default.join(ARCHIVE).is_file());
}

#[test]
fn an_empty_mdbook_leaves_the_install_in_the_default_folder_under_the_home() {
    let tool = Tool::new();
    let (home, default) = tool.open_home();
    let run = tool.install(&home, Some(""), &[]);
    assert_installed_under(&run, &default);
    assert!(default.join(ARCHIVE).is_file());
}

#[test]
fn without_mdbook_or_with_it_empty_the_book_build_uses_the_default_folder_under_the_home() {
    for folder in [None, Some("")] {
        let tool = Tool::new();
        let (home, default) = tool.open_home();
        tool.fill(&default);
        tool.offline();
        let book = tool.path("book");
        let run = tool.build(&book, &home, folder, &[]);
        assert_eq!(run.code, Some(0), "MDBOOK {folder:?}: {}", run.out);
        assert!(book.join("index.html").is_file(), "MDBOOK {folder:?}: no book: {}", run.out);
    }
}

// ---- The binary in the cache: held for installed only as the pinned archive gives it ----

/// A cache folder as a successful install leaves it, its address cut, and the binary in it.
#[cfg(unix)]
fn filled() -> (Tool, PathBuf, PathBuf) {
    let tool = Tool::new();
    let folder = tool.path("shared-tools");
    let run = tool.install(&tool.sealed_home(), Some(text(&folder)), &[]);
    assert_installed_under(&run, &folder);
    tool.offline();
    let binary = PathBuf::from(run.stdout.lines().last().unwrap().trim());
    (tool, folder, binary)
}

#[cfg(unix)]
fn mode_of(path: &Path) -> u32 {
    use std::os::unix::fs::PermissionsExt;
    std::fs::metadata(path).unwrap().permissions().mode() & 0o777
}

#[cfg(unix)]
fn inode_of(path: &Path) -> u64 {
    use std::os::unix::fs::MetadataExt;
    std::fs::metadata(path).unwrap().ino()
}

/// Leaves the binary of the cache holding these bytes under this mode.
#[cfg(unix)]
fn leave(binary: &Path, bytes: &[u8], mode: u32) {
    use std::os::unix::fs::PermissionsExt;
    std::fs::write(binary, bytes).unwrap();
    std::fs::set_permissions(binary, std::fs::Permissions::from_mode(mode)).unwrap();
}

/// The binaries that are not the installed tool, each with what it is, its bytes and its mode.
#[cfg(unix)]
fn damaged_binaries() -> Vec<(&'static str, Vec<u8>, u32)> {
    let whole = FAKE_MDBOOK.as_bytes();
    let mut altered = whole.to_vec();
    *altered.iter_mut().rev().nth(1).unwrap() ^= 1;
    let mut longer = whole.to_vec();
    longer.extend_from_slice(b"exit 1\n");
    vec![
        ("truncated", whole[..whole.len() / 2].to_vec(), 0o755),
        ("short of its last byte", whole[..whole.len() - 1].to_vec(), 0o755),
        ("empty", Vec::new(), 0o755),
        ("of the same length with one byte changed", altered, 0o755),
        ("followed by more bytes", longer, 0o755),
        ("complete and not executable", whole.to_vec(), 0o644),
        ("truncated and not executable", whole[..whole.len() / 2].to_vec(), 0o600),
    ]
}

/// The install gives the pinned binary, executable, at the path it had.
#[cfg(unix)]
fn assert_pinned_binary(run: &Run, folder: &Path, binary: &Path, case: &str) {
    assert_eq!(run.code, Some(0), "a binary {case}: {}", run.out);
    assert_installed_under(run, folder);
    assert_eq!(Path::new(run.stdout.lines().last().unwrap().trim()), binary, "a binary {case}");
    assert_eq!(
        String::from_utf8_lossy(&std::fs::read(binary).unwrap()),
        FAKE_MDBOOK,
        "a binary {case} is taken as installed"
    );
    assert_eq!(mode_of(binary) & 0o111, 0o111, "a binary {case}: the installed binary is not executable");
}

#[test]
fn the_pin_holds_no_checksum_but_that_of_the_archive() {
    let keys: BTreeSet<String> = read(PIN)
        .lines()
        .filter(|line| !line.trim_start().starts_with('#'))
        .filter_map(|line| line.split_once('=').map(|(key, _)| key.trim().to_string()))
        .collect();
    assert_eq!(keys, ["sha256", "url", "version"].map(String::from).into());
}

#[cfg(unix)]
#[test]
fn a_binary_that_is_not_the_member_of_the_pinned_archive_or_not_executable_is_replaced_by_it() {
    for (case, bytes, mode) in damaged_binaries() {
        let (tool, folder, binary) = filled();
        leave(&binary, &bytes, mode);
        let before = files_under(&folder);
        let run = tool.install(&tool.sealed_home(), Some(text(&folder)), &[]);
        assert_pinned_binary(&run, &folder, &binary, case);
        assert_eq!(files_under(&folder), before, "a binary {case}: the install leaves a temporary file");
    }
}

#[cfg(unix)]
#[test]
fn a_damaged_binary_is_replaced_by_renaming_a_new_file_over_it() {
    for (case, bytes, mode) in damaged_binaries() {
        let (tool, folder, binary) = filled();
        leave(&binary, &bytes, mode);
        let link = tool.path("link-taken-before");
        std::fs::hard_link(&binary, &link).unwrap();
        let run = tool.install(&tool.sealed_home(), Some(text(&folder)), &[]);
        assert_pinned_binary(&run, &folder, &binary, case);
        assert_eq!(std::fs::read(&link).unwrap(), bytes, "a binary {case}: the old file is written in place");
        assert_eq!(mode_of(&link), mode, "a binary {case}: the mode of the old file is changed");
        assert_ne!(inode_of(&binary), inode_of(&link), "a binary {case}: the old file stays in place");
    }
}

#[cfg(unix)]
#[test]
fn a_first_install_leaves_the_archive_and_the_executable_binary_and_nothing_else() {
    let (_tool, folder, binary) = filled();
    let rel = binary.strip_prefix(&folder).unwrap().to_string_lossy().to_string();
    assert_eq!(files_under(&folder), [ARCHIVE.to_string(), rel].into());
    assert_eq!(std::fs::read(&binary).unwrap(), FAKE_MDBOOK.as_bytes());
    assert_eq!(mode_of(&binary) & 0o111, 0o111);
}

/// What killed installs may leave beside the archive and beside the binary: files that another
/// install may still be writing.
#[cfg(unix)]
fn leftovers(folder: &Path, binary: &Path) -> Vec<(PathBuf, Vec<u8>)> {
    let beside = binary.parent().unwrap();
    let paths = [
        folder.join("tmpk1ll3d00.part"),
        beside.join("tmpk1ll3d01.part"),
        beside.join("tmpk1ll3d02"),
        beside.join("mdbook.part"),
        beside.join("mdbook.tmp"),
        beside.join(".mdbook.part"),
    ];
    paths
        .into_iter()
        .map(|path| {
            let bytes = FAKE_MDBOOK.as_bytes()[..FAKE_MDBOOK.len() / 3].to_vec();
            leave(&path, &bytes, 0o755);
            (path, bytes)
        })
        .collect()
}

#[cfg(unix)]
#[test]
fn the_temporary_files_of_a_killed_install_are_kept_and_do_not_stop_the_replacement_of_the_binary() {
    for (case, bytes, mode) in damaged_binaries() {
        let (tool, folder, binary) = filled();
        leave(&binary, &bytes, mode);
        let left = leftovers(&folder, &binary);
        let before = files_under(&folder);
        let run = tool.install(&tool.sealed_home(), Some(text(&folder)), &[]);
        assert_pinned_binary(&run, &folder, &binary, case);
        for (path, bytes) in left {
            assert_eq!(std::fs::read(&path).ok(), Some(bytes), "a binary {case}: {path:?} is touched");
        }
        assert_eq!(files_under(&folder), before, "a binary {case}: the files of the folder changed");
    }
}

#[cfg(unix)]
#[test]
fn the_temporary_files_of_a_killed_install_are_never_taken_for_the_missing_binary() {
    let (tool, folder, binary) = filled();
    std::fs::remove_file(&binary).unwrap();
    let left = leftovers(&folder, &binary);
    let mut expected = files_under(&folder);
    expected.insert(binary.strip_prefix(&folder).unwrap().to_string_lossy().to_string());
    let run = tool.install(&tool.sealed_home(), Some(text(&folder)), &[]);
    assert_pinned_binary(&run, &folder, &binary, "missing");
    for (path, bytes) in left {
        assert_eq!(std::fs::read(&path).ok(), Some(bytes), "{path:?} is touched");
    }
    assert_eq!(files_under(&folder), expected);
}

#[cfg(unix)]
#[test]
fn the_book_build_replaces_a_truncated_binary_and_writes_the_book() {
    let (tool, folder, binary) = filled();
    leave(&binary, &FAKE_MDBOOK.as_bytes()[..FAKE_MDBOOK.find("&&").unwrap()], 0o755);
    let book = tool.path("book");
    let run = tool.build(&book, &tool.sealed_home(), Some(text(&folder)), &[]);
    assert_eq!(run.code, Some(0), "{}", run.out);
    assert!(book.join("index.html").is_file(), "no book written: {}", run.out);
    assert_eq!(std::fs::read(&binary).unwrap(), FAKE_MDBOOK.as_bytes());
}

#[cfg(unix)]
#[test]
fn an_intact_executable_binary_is_not_rewritten_by_a_new_install() {
    for mode in [0o755, 0o700] {
        let (tool, folder, binary) = filled();
        leave(&binary, FAKE_MDBOOK.as_bytes(), mode);
        let then = std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1_000_000_000);
        std::fs::File::options().write(true).open(&binary).unwrap().set_modified(then).unwrap();
        let inode = inode_of(&binary);
        let before = files_under(&folder);
        for _ in 0..2 {
            let run = tool.install(&tool.sealed_home(), Some(text(&folder)), &[]);
            assert_installed_under(&run, &folder);
        }
        assert_eq!(inode_of(&binary), inode, "mode {mode:o}: another file is at the path of the binary");
        assert_eq!(std::fs::metadata(&binary).unwrap().modified().unwrap(), then, "mode {mode:o}: rewritten");
        assert_eq!(files_under(&folder), before, "mode {mode:o}");
    }
}

#[test]
fn an_intact_binary_beside_an_archive_that_differs_from_the_pin_is_not_taken_as_installed() {
    let tool = Tool::new();
    let folder = tool.path("shared-tools");
    tool.fill(&folder);
    tool.offline();
    write(&folder, ARCHIVE, "another archive");
    let run = tool.install(&tool.sealed_home(), Some(text(&folder)), &[]);
    assert_eq!(run.code, Some(1), "{}", run.out);
    assert!(!run.stdout.lines().any(|l| l.trim().ends_with("mdbook")), "a binary path is printed: {}", run.stdout);
}

#[cfg(unix)]
#[test]
fn a_read_only_folder_whose_binary_is_truncated_or_altered_is_refused_with_the_command_that_fills_it() {
    for (case, bytes, mode) in damaged_binaries() {
        if mode & 0o111 == 0 {
            continue;
        }
        println!("a binary {case}");
        let (tool, folder, binary) = filled();
        leave(&binary, &bytes, mode);
        assert_read_only_folder_refused(&tool, &folder);
    }
}

#[cfg(unix)]
#[test]
fn a_read_only_folder_whose_binary_is_not_executable_is_refused_with_the_command_that_fills_it() {
    for (case, bytes, mode) in damaged_binaries() {
        if mode & 0o111 != 0 {
            continue;
        }
        println!("a binary {case}");
        let (tool, folder, binary) = filled();
        leave(&binary, &bytes, mode);
        assert_read_only_folder_refused(&tool, &folder);
    }
}

// ---- The link check, on fixtures ----

const GOOD_LIST: &str = "# a fixture list\nCONTRIBUTING.md\nexamples/\nschemas/\nspec/\n";

struct Book {
    dir: TempDir,
}

impl Book {
    fn new(pages: &[(&str, &str)]) -> Book {
        let dir = tempfile::tempdir().unwrap();
        for (rel, text) in pages {
            write(&dir.path().join("book"), rel, text);
        }
        write(&dir.path().join("site"), "schemas/0.6/device.schema.json", "{}");
        write(dir.path(), "public.list", GOOD_LIST);
        Book { dir }
    }

    fn check(&self) -> Run {
        let root = self.dir.path();
        python(
            CHECK_BOOK,
            &[
                root.join("book").to_str().unwrap(),
                "--site",
                root.join("site").to_str().unwrap(),
                "--public-list",
                root.join("public.list").to_str().unwrap(),
            ],
        )
    }
}

fn page(links: &[&str]) -> String {
    let body: String = links.iter().map(|l| format!("<a href=\"{l}\">x</a>\n")).collect();
    format!("<html><body><h1 id=\"top\">T</h1>\n{body}</body></html>")
}

const SECOND: &str = "<html><body><h2 id=\"part\">P</h2></body></html>";

#[test]
fn a_book_whose_links_all_resolve_passes_the_link_check() {
    let book = Book::new(&[
        (
            "index.html",
            &page(&[
                "second.html",
                "second.html#part",
                "#top",
                "../schemas/0.6/device.schema.json",
                &format!("{REPO}/blob/main/CONTRIBUTING.md"),
                &format!("{REPO}/tree/main/examples/0.6"),
                "https://example.org/elsewhere",
            ]),
        ),
        ("second.html", SECOND),
    ]);
    let run = book.check();
    assert_eq!(run.code, Some(0), "{}", run.out);
}

#[test]
fn a_link_to_a_page_the_book_does_not_hold_fails_naming_the_page_and_the_target() {
    let book = Book::new(&[("index.html", &page(&["missing-page.html"])), ("second.html", SECOND)]);
    let run = book.check();
    assert_eq!(run.code, Some(1), "{}", run.out);
    assert!(run.out.contains("index.html") && run.out.contains("missing-page.html"), "{}", run.out);
}

#[test]
fn an_anchor_that_no_heading_of_the_page_carries_fails_naming_the_page_and_the_target() {
    let book = Book::new(&[("index.html", &page(&["second.html#nowhere"])), ("second.html", SECOND)]);
    let run = book.check();
    assert_eq!(run.code, Some(1), "{}", run.out);
    assert!(run.out.contains("index.html") && run.out.contains("second.html#nowhere"), "{}", run.out);
}

#[test]
fn a_link_to_a_schema_the_site_does_not_hold_fails_naming_the_page_and_the_target() {
    let book = Book::new(&[("index.html", &page(&["../schemas/0.6/nope.schema.json"])), ("second.html", SECOND)]);
    let run = book.check();
    assert_eq!(run.code, Some(1), "{}", run.out);
    assert!(run.out.contains("index.html") && run.out.contains("nope.schema.json"), "{}", run.out);
}

#[test]
fn a_link_to_a_workshop_only_file_of_the_repository_fails_naming_the_page_and_the_target() {
    let charter = concat!("CLAUDE", ".md");
    let target = format!("{REPO}/blob/main/{charter}");
    let book = Book::new(&[("index.html", &page(&[&target])), ("second.html", SECOND)]);
    let run = book.check();
    assert_eq!(run.code, Some(1), "{}", run.out);
    assert!(run.out.contains("index.html") && run.out.contains(charter), "{}", run.out);
}

// ---- The real book ----

/// The site folder and the book folder, side by side in a folder the test removes.
fn built() -> (TempDir, PathBuf, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let site = dir.path().join("site");
    let book = dir.path().join("book");
    let run = python(BUILD_SITE, &[site.to_str().unwrap()]);
    assert_eq!(run.code, Some(0), "{BUILD_SITE}: {}", run.out);
    let before = files_under(&site);
    let run = python(BUILD_BOOK, &[book.to_str().unwrap()]);
    assert_eq!(run.code, Some(0), "{BUILD_BOOK} failed:\n{}", run.out);
    assert_eq!(files_under(&site), before, "the book build wrote into the site folder");
    (dir, site, book)
}

fn text_of(html: &str) -> String {
    let mut out = String::new();
    let mut in_tag = false;
    for c in html.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    out.replace("&amp;", "&").replace("&lt;", "<").replace("&gt;", ">").replace("&quot;", "\"").replace("&#39;", "'")
}

fn hrefs(html: &str) -> Vec<String> {
    html.split("href=\"").skip(1).map(|s| s.split('"').next().unwrap().replace("&amp;", "&")).collect()
}

#[test]
fn the_book_holds_the_text_of_the_spec_one_page_per_text_and_one_reference_page_per_kind() {
    let (_dir, _site, book) = built();
    let page = std::fs::read_to_string(book.join("0.6.html")).expect("the page of 0.6");
    let text = text_of(&page);
    let spec = read("spec/0.6.md");
    let mut in_code = false;
    let mut headings = 0;
    for line in spec.lines() {
        if line.starts_with("```") {
            in_code = !in_code;
        }
        if in_code || !line.starts_with('#') {
            continue;
        }
        let heading = line.trim_start_matches('#').trim().replace('`', "");
        headings += 1;
        assert!(text.contains(&heading), "the page of 0.6 lacks the heading {heading:?}");
    }
    assert!(headings > 20, "{headings} headings read");
    let sentence = "The suite checks itself: the examples and the inputs of every conformance case";
    assert!(spec.contains(sentence));
    assert!(text.contains(sentence), "the page of 0.6 lacks {sentence:?}");
    for rel in ["PROCESS.html", "ERRATA.html", "CHANGELOG.html", "vocabulary/1.html"] {
        assert!(book.join(rel).is_file(), "{rel} is not a page of the book");
    }
    for kind in ["device", "profile", "sound", "fingerprint", "firmware", "assembly"] {
        let rel = format!("reference/{kind}.html");
        assert!(book.join(&rel).is_file(), "{rel} is not a page of the book");
    }
}

#[test]
fn links_of_the_spec_to_schemas_and_to_repository_files_are_rendered_at_their_addresses() {
    let (_dir, _site, book) = built();
    let links = hrefs(&std::fs::read_to_string(book.join("0.6.html")).unwrap());
    let schema = links.iter().find(|l| l.ends_with("schemas/0.6/device.schema.json")).expect("the device schema link");
    assert!(!schema.starts_with("http"), "a schema link leaves the site: {schema}");
    for tail in ["schemas/extensions.md", "conformance/README.md", "CONTRIBUTING.md"] {
        let want = format!("{REPO}/blob/main/{tail}");
        assert!(links.contains(&want), "no link {want} in {links:?}");
    }
    assert!(
        links.iter().any(|l| l.starts_with(&format!("{REPO}/")) && l.ends_with("main/examples/0.6")),
        "no link to examples/0.6 in the repository"
    );
    let spec = read("spec/0.6.md");
    assert!(spec.contains("](../schemas/extensions.md)"), "the spec text is rewritten");
}

#[test]
fn the_link_check_is_green_on_the_real_book_and_the_site_stays_as_it_was() {
    let (_dir, site, book) = built();
    let list = common::repo().join("export/public.list");
    let mut args = vec![book.to_str().unwrap().to_string(), "--site".into(), site.to_str().unwrap().to_string()];
    if list.is_file() {
        args.push("--public-list".into());
        args.push(list.to_str().unwrap().to_string());
    }
    let refs: Vec<&str> = args.iter().map(String::as_str).collect();
    let run = python(CHECK_BOOK, &refs);
    assert_eq!(run.code, Some(0), "{}", run.out);
    let run = python("tools/check_site.py", &["--site", site.to_str().unwrap()]);
    assert_eq!(run.code, Some(0), "{}", run.out);
}

/// The site folder with the book written under its `book/` folder, as the pages workflow uploads it.
type SiteFiles = Vec<(String, Vec<u8>)>;

fn built_with_book() -> (TempDir, PathBuf, BTreeSet<String>, SiteFiles) {
    let dir = tempfile::tempdir().unwrap();
    let site = dir.path().join("site");
    let run = python(BUILD_SITE, &[site.to_str().unwrap()]);
    assert_eq!(run.code, Some(0), "{BUILD_SITE}: {}", run.out);
    let before = files_under(&site);
    let bytes: SiteFiles =
        before.iter().map(|f| (f.clone(), std::fs::read(site.join(f)).unwrap())).collect();
    let run = python(BUILD_BOOK, &[site.join("book").to_str().unwrap()]);
    assert_eq!(run.code, Some(0), "{BUILD_BOOK} failed:\n{}", run.out);
    (dir, site, before, bytes)
}

#[test]
fn the_site_that_holds_the_book_keeps_every_file_of_the_site_built_alone_at_its_path_with_its_bytes() {
    let (_dir, site, before, bytes) = built_with_book();
    let after = files_under(&site);
    assert!(before.is_subset(&after));
    for (rel, old) in bytes {
        assert_eq!(std::fs::read(site.join(&rel)).unwrap(), old, "{rel} changed");
    }
    assert!(after.contains("book/index.html"), "no book under book/: {after:?}");
    assert!(before.contains("schemas/SHA256SUMS") && before.contains("schemas/extensions.md"));
    assert!(after.iter().filter(|f| !before.contains(*f)).all(|f| f.starts_with("book/")));
}

#[test]
fn the_link_check_and_the_site_check_are_green_on_the_book_written_under_the_site() {
    let (_dir, site, _before, _bytes) = built_with_book();
    let book = site.join("book");
    let list = common::repo().join("export/public.list");
    let mut args = vec![book.to_str().unwrap(), "--site", site.to_str().unwrap()];
    if list.is_file() {
        args.push("--public-list");
        args.push(list.to_str().unwrap());
    }
    let run = python(CHECK_BOOK, &args);
    assert_eq!(run.code, Some(0), "{}", run.out);
    let run = python("tools/check_site.py", &["--site", site.to_str().unwrap()]);
    assert_eq!(run.code, Some(0), "{}", run.out);
}

#[test]
fn the_links_of_the_book_to_a_schema_resolve_to_the_schema_path_in_the_site() {
    let (_dir, site, _before, _bytes) = built_with_book();
    let spec = std::fs::read_to_string(site.join("book/0.6.html")).unwrap();
    let mut seen = 0;
    for href in hrefs(&spec).iter().filter(|h| h.contains("schemas/") && !h.starts_with("http")) {
        let path = href.split('#').next().unwrap();
        let resolved = site.join("book").join(path);
        assert!(resolved.canonicalize().map(|p| p.is_file()).unwrap_or(false), "{href} resolves to no file");
        assert!(resolved.canonicalize().unwrap().starts_with(site.canonicalize().unwrap().join("schemas")));
        seen += 1;
    }
    assert!(seen > 0, "no schema link in the book");
}

#[test]
fn the_home_page_links_the_book_relatively_and_the_readme_links_it_by_the_site_address() {
    let dir = tempfile::tempdir().unwrap();
    let site = dir.path().join("site");
    let run = python(BUILD_SITE, &[site.to_str().unwrap()]);
    assert_eq!(run.code, Some(0), "{BUILD_SITE}: {}", run.out);
    let home = std::fs::read_to_string(site.join("index.html")).unwrap();
    assert!(home.contains("href=\"book/index.html\""), "home page: {home}");
    assert!(
        read("README.md").contains("https://roomi-fields.github.io/open-synth-descriptors/book/"),
        "README does not link the book"
    );
}

// ---- The candidates ----

/// A fake book tool that publishes the source it is given, so a test reads the pages assembled.
const SOURCE_MDBOOK: &str = "#!/bin/sh\nmkdir -p \"$4\" && cp -r \"$2/src/.\" \"$4/\"\n";

/// A fake reference generator whose one page names the folder of the schemas it is given.
const NAMING_REFERENCE: &str = "#!/bin/sh\nmkdir -p \"$3\" && echo \"# Device of ${2##*/}\" > \"$3/device.md\"\n";

/// The source of the book that the script `script` assembles, with the list of candidates `list`
/// given by `--candidates`, or without that option.
fn assembled(script: &Path, list: Option<&str>) -> (Tool, PathBuf) {
    let tool = Tool::holding(SOURCE_MDBOOK);
    let reference = tool.path("naming-reference");
    executable(&reference, NAMING_REFERENCE);
    let candidates = tool.path("candidates.list");
    let book = tool.path("book");
    let cache = tool.path("cache");
    let mut args = vec![text(&book), "--pin", text(&tool.pin), "--cache", text(&cache)];
    if let Some(list) = list {
        std::fs::write(&candidates, list).unwrap();
        args.extend(["--candidates", text(&candidates)]);
    }
    let run = python_with(script.to_str().unwrap(), &args, &[("OSD_REFERENCE", Some(text(&reference)))]);
    assert_eq!(run.code, Some(0), "{BUILD_BOOK}: {}", run.out);
    (tool, book)
}

fn page_text(book: &Path, rel: &str) -> String {
    std::fs::read_to_string(book.join(rel)).unwrap_or_else(|e| panic!("{rel} of the book: {e}"))
}

/// A repository with the scripts of `tools/`, the versions 1.0 and 2.0, a table beside them, and
/// 2.0 listed as candidate with that table.
fn repository_of_two_versions() -> TempDir {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    std::fs::create_dir_all(root.join("tools")).unwrap();
    for entry in std::fs::read_dir(common::repo().join("tools")).unwrap() {
        let path = entry.unwrap().path();
        if path.is_file() {
            std::fs::copy(&path, root.join("tools").join(path.file_name().unwrap())).unwrap();
        }
    }
    write(root, "tools/candidates.list", TWO);
    write(
        root,
        "spec/1.0.md",
        "# One\n\n[next](2.0.md) [part](2.0.md#part) [device](../schemas/2.0/device.schema.json) \
         [table](../schemas/table.json) [own](../schemas/1.0/device.schema.json)\n",
    );
    write(root, "spec/2.0.md", "# Two\n\n## Part\n\n[before](1.0.md)\n");
    write(root, "spec/vocabulary/1.md", "# Words\n\n[next](../2.0.md) [table](../../schemas/table.json)\n");
    for rel in ["schemas/1.0/device.schema.json", "schemas/2.0/device.schema.json", "schemas/table.json", "schemas/vocabulary/1/terms.json"] {
        write(root, rel, "{}\n");
    }
    dir
}

const TWO: &str = "# the candidates\n2.0 schemas/table.json\n";

fn public(path: &str) -> String {
    format!("]({REPO}/blob/main/{path})")
}

#[test]
fn the_book_has_no_page_of_a_candidate_and_the_reference_of_the_newest_other_version() {
    let root = repository_of_two_versions();
    for list in [None, Some(TWO)] {
        let (_tool, book) = assembled(&root.path().join(BUILD_BOOK), list);
        assert!(!book.join("2.0.md").exists(), "the book has the page of the candidate");
        assert!(book.join("1.0.md").is_file() && book.join("vocabulary/1.md").is_file());
        let summary = page_text(&book, "SUMMARY.md");
        assert!(!summary.contains("(2.0.md)"), "{summary}");
        assert!(summary.contains("# Reference 1.0"), "{summary}");
        assert!(page_text(&book, "reference/device.md").contains("Device of 1.0"));
    }
}

#[test]
fn a_link_to_a_page_or_a_schema_withheld_is_rendered_at_its_address_in_the_public_repository() {
    let root = repository_of_two_versions();
    let (_tool, book) = assembled(&root.path().join(BUILD_BOOK), None);
    let one = page_text(&book, "1.0.md");
    for want in [
        public("spec/2.0.md"),
        public("spec/2.0.md#part"),
        public("schemas/2.0/device.schema.json"),
        public("schemas/table.json"),
    ] {
        assert!(one.contains(&want), "no {want} in {one}");
    }
    assert!(one.contains("](../schemas/1.0/device.schema.json)"), "a schema the site holds leaves it: {one}");
    let words = page_text(&book, "vocabulary/1.md");
    for want in [public("spec/2.0.md"), public("schemas/table.json")] {
        assert!(words.contains(&want), "no {want} in {words}");
    }
}

#[test]
fn with_no_candidate_the_book_has_every_page_and_the_reference_of_the_newest_version() {
    let root = repository_of_two_versions();
    let (_tool, book) = assembled(&root.path().join(BUILD_BOOK), Some("# none\n"));
    assert!(book.join("2.0.md").is_file());
    assert!(page_text(&book, "SUMMARY.md").contains("# Reference 2.0"));
    assert!(page_text(&book, "reference/device.md").contains("Device of 2.0"));
    let one = page_text(&book, "1.0.md");
    for kept in ["](2.0.md)", "](2.0.md#part)", "](../schemas/2.0/device.schema.json)", "](../schemas/table.json)"] {
        assert!(one.contains(kept), "no {kept} in {one}");
    }
}

#[test]
fn an_older_candidate_leaves_the_book_and_its_newer_versions_stay() {
    let root = repository_of_two_versions();
    let (_tool, book) = assembled(&root.path().join(BUILD_BOOK), Some("1.0\n"));
    assert!(!book.join("1.0.md").exists());
    assert!(page_text(&book, "reference/device.md").contains("Device of 2.0"));
    let two = page_text(&book, "2.0.md");
    assert!(two.contains(&public("spec/1.0.md")), "{two}");
}

#[test]
fn the_book_of_the_repository_with_0_7_listed_has_no_page_of_it_and_the_reference_of_0_6() {
    let list = "0.7 schemas/providers.json schemas/providers.schema.json\n";
    let (_tool, book) = assembled(&common::repo().join(BUILD_BOOK), Some(list));
    assert!(!book.join("0.7.md").exists());
    assert!(book.join("0.6.md").is_file() && book.join("CHANGELOG.md").is_file());
    assert!(page_text(&book, "reference/device.md").contains("Device of 0.6"));
}

#[test]
fn the_book_of_the_repository_without_the_option_has_no_page_of_a_listed_version() {
    let list = read("tools/candidates.list");
    let (_tool, book) = assembled(&common::repo().join(BUILD_BOOK), None);
    for line in list.lines().map(str::trim).filter(|l| !l.is_empty() && !l.starts_with('#')) {
        let version = line.split_whitespace().next().unwrap();
        assert!(!book.join(format!("{version}.md")).exists(), "the book has the page of {version}");
    }
    assert!(book.join("0.6.md").is_file());
}

#[test]
fn with_no_candidate_the_real_book_has_the_page_of_0_7_and_its_links_resolve_in_the_whole_site() {
    let dir = tempfile::tempdir().unwrap();
    let candidates = dir.path().join("candidates.list");
    std::fs::write(&candidates, "# none\n").unwrap();
    let site = dir.path().join("site");
    let book = site.join("book");
    let run = python(BUILD_SITE, &[text(&site), "--candidates", text(&candidates)]);
    assert_eq!(run.code, Some(0), "{BUILD_SITE}: {}", run.out);
    let run = python(BUILD_BOOK, &[text(&book), "--candidates", text(&candidates)]);
    assert_eq!(run.code, Some(0), "{BUILD_BOOK}: {}", run.out);
    assert!(book.join("0.7.html").is_file(), "no page of 0.7");
    let mut args = vec![text(&book), "--site", text(&site)];
    let list = common::repo().join("export/public.list");
    if list.is_file() {
        args.extend(["--public-list", text(&list)]);
    }
    let run = python(CHECK_BOOK, &args);
    assert_eq!(run.code, Some(0), "{}", run.out);
}

// ---- The workflows ----

fn run_lines(rel: &str) -> Vec<String> {
    let wf: serde_yaml_ng::Value = serde_yaml_ng::from_str(&read(rel)).unwrap();
    let mut out = Vec::new();
    for job in wf["jobs"].as_mapping().unwrap().values() {
        for step in job.get("steps").and_then(|s| s.as_sequence()).cloned().unwrap_or_default() {
            if let Some(run) = step.get("run").and_then(|r| r.as_str()) {
                out.extend(run.lines().map(str::trim).filter(|l| !l.is_empty()).map(String::from));
            }
        }
    }
    out
}

fn position(lines: &[String], needle: &str) -> usize {
    lines.iter().position(|l| l.contains(needle)).unwrap_or_else(|| panic!("no step runs {needle}: {lines:?}"))
}

#[test]
fn the_pages_and_check_workflows_build_the_reference_install_the_tool_build_the_book_and_check_its_links() {
    for rel in [PAGES, CHECK] {
        let lines = run_lines(rel);
        let cargo = lines
            .iter()
            .position(|l| l.starts_with("cargo") && l.contains("reference"))
            .unwrap_or_else(|| panic!("{rel}: no cargo step builds the reference: {lines:?}"));
        let install = position(&lines, INSTALL);
        let build = position(&lines, BUILD_BOOK);
        let check = position(&lines, CHECK_BOOK);
        assert!(cargo < build && install < build && build < check, "{rel}: order {lines:?}");
    }
}

#[test]
fn the_pages_workflow_uploads_the_site_folder_that_holds_the_book_under_book_and_checks_it_there() {
    let wf: serde_yaml_ng::Value = serde_yaml_ng::from_str(&read(PAGES)).unwrap();
    let upload = wf["jobs"]["build"]["steps"]
        .as_sequence()
        .unwrap()
        .iter()
        .find(|s| s.get("uses").and_then(|u| u.as_str()).is_some_and(|u| u.starts_with("actions/upload-pages-artifact")))
        .expect("an upload step");
    assert_eq!(upload["with"]["path"].as_str(), Some("_site"));
    let lines = run_lines(PAGES);
    let build = &lines[position(&lines, BUILD_BOOK)];
    assert!(build.split_whitespace().any(|w| w == "_site/book"), "the book is not written to _site/book: {build}");
    let site = position(&lines, BUILD_SITE);
    assert!(site < position(&lines, BUILD_BOOK), "the site is built before the book is written into it");
    let check = &lines[position(&lines, CHECK_BOOK)];
    assert!(check.split_whitespace().any(|w| w == "_site/book"), "the check reads another folder: {check}");
    assert!(check.contains("--site _site"), "{check}");
}
