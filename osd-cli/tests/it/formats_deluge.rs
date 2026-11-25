//! `osd export deluge` writes a Deluge device definition and says its losses; `osd import
//! deluge` is refused because the Deluge does not import.

use crate::common::{osd, repo};

fn minilogue() -> String {
    repo().join("examples/0.6/minilogue.device.json").to_string_lossy().into_owned()
}

#[test]
fn export_writes_the_midi_device_document_with_its_labels_and_credit_comment() {
    let run = osd(&["export", "deluge", &minilogue()]);
    assert_eq!(run.code, 0, "{:?}", run.stderr);
    assert!(run.stdout.contains("<midiDevice>"), "{}", run.stdout);
    assert!(run.stdout.contains("<ccLabels"), "{}", run.stdout);
    assert!(run.stdout.contains("<!--"), "{}", run.stdout);
    assert!(run.stdout.trim_end().ends_with("</midiDevice>"), "{}", run.stdout);
}

#[test]
fn export_prints_each_loss_on_the_error_output_then_the_credit_to_carry() {
    let run = osd(&["export", "deluge", &minilogue()]);
    let lines: Vec<&str> = run.stderr.lines().collect();
    let credit = lines.iter().position(|l| *l == "credit to carry with the file:");
    let (losses, block) = lines.split_at(credit.unwrap_or(lines.len()));
    assert!(losses.iter().all(|l| l.starts_with('/') && l.contains(": ")), "{:?}", run.stderr);
    assert!(losses.iter().any(|l| l.starts_with("/sources: ")), "{:?}", run.stderr);
    assert!(credit.is_some(), "{:?}", run.stderr);
    assert!(block.len() > 1 && block[1..].iter().all(|l| l.starts_with("  ")), "{:?}", run.stderr);
}

#[test]
fn import_of_the_deluge_exits_1_and_says_the_deluge_is_not_imported() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("def.XML").to_string_lossy().into_owned();
    std::fs::write(&file, "<midiDevice><ccLabels 1=\"A\"/></midiDevice>").unwrap();
    let run = osd(&[
        "import", "deluge", &file, "--source-url", "https://example.org/x", "--source-author", "A maker", "--license", "CC-BY-4.0",
    ]);
    assert_eq!(run.code, 1, "{:?}", run.stderr);
    assert_eq!(run.stdout, "");
    let err = run.stderr.to_lowercase();
    assert!(err.contains("deluge") && err.contains("import"), "{:?}", run.stderr);
}

#[test]
fn the_unknown_format_message_and_the_helps_list_deluge() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("x.json").to_string_lossy().into_owned();
    std::fs::write(&file, "{}").unwrap();
    for word in ["import", "export"] {
        let run = osd(&[word, "no-such-format", &file]);
        assert_eq!(run.code, 2, "{word}: {:?}", run.stderr);
        assert!(run.stderr.contains("deluge"), "{word}: {:?}", run.stderr);
        let help = osd(&[word, "--help"]);
        assert!(help.stdout.contains("deluge") || help.stderr.contains("deluge"), "{word}");
    }
    assert!(osd(&["--help"]).stdout.contains("deluge"));
}
