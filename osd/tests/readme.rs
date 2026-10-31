//! The README announces the current version and links its schemas (OSD-b91.2.9).

mod common;

fn readme() -> String {
    std::fs::read_to_string(common::repo().join("README.md")).unwrap()
}

#[test]
fn criterion_5_the_readme_states_the_status_as_0_5_draft() {
    assert!(readme().contains("**Status: 0.5, draft.**"));
}

#[test]
fn criterion_5_the_five_schema_links_of_the_table_point_to_schemas_0_5() {
    let text = readme();
    for kind in ["device", "profile", "sound", "fingerprint", "firmware"] {
        let link = format!("[`{kind}.schema.json`](schemas/0.5/{kind}.schema.json)");
        assert!(text.contains(&link), "missing {link}");
        let old = format!("](schemas/0.4/{kind}.schema.json)");
        assert!(!text.contains(&old), "{kind} still links 0.4");
    }
}
