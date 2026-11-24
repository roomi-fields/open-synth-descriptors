//! The words `count` and `undescribed` of the program.

use super::build::{run, SCHEMAS};

fn lines(args: &[&str]) -> Vec<String> {
    let res = run(args);
    assert_eq!(res.status.code(), Some(0), "{}", String::from_utf8_lossy(&res.stderr));
    String::from_utf8(res.stdout).unwrap().lines().map(str::to_string).collect()
}

#[test]
fn count_prints_one_line_per_kind_and_the_device_reads_144_of_296() {
    let l = lines(&["count", SCHEMAS]);
    assert_eq!(l.len(), 6, "{l:?}");
    assert!(l.contains(&"device: 144 of 296".to_string()), "{l:?}");
    for k in ["profile", "sound", "fingerprint", "firmware", "assembly"] {
        assert!(l.iter().any(|x| x.starts_with(&format!("{k}: "))), "{k}");
    }
}

#[test]
fn undescribed_lists_one_sorted_line_per_missing_description() {
    let l = lines(&["undescribed", SCHEMAS]);
    let mut sorted = l.clone();
    sorted.sort();
    assert_eq!(l, sorted);
    assert_eq!(l.iter().filter(|x| x.starts_with("device ")).count(), 296 - 144);
    assert!(l.iter().all(|x| x.split(' ').count() == 2), "<kind> <pointer>");
}

#[test]
fn undescribed_gives_the_same_list_twice() {
    assert_eq!(lines(&["undescribed", SCHEMAS]), lines(&["undescribed", SCHEMAS]));
}
