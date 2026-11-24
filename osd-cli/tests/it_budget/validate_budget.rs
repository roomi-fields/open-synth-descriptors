//! docs/CADRE.md R12: validating one descriptor takes under 100 ms, start of
//! the program included. A time budget: played at rest, by name; a failure
//! under load is to be replayed.

use crate::common::{example_path, osd};
use std::time::{Duration, Instant};

#[test]
fn validating_one_example_descriptor_takes_under_100_ms_start_included() {
    let path = example_path("device").to_string_lossy().into_owned();
    let best = (0..5)
        .map(|_| {
            let start = Instant::now();
            let run = osd(&["validate", &path]);
            let took = start.elapsed();
            assert_eq!(run.code, 0, "{:?}", run.stderr);
            took
        })
        .min()
        .unwrap();
    assert!(best < Duration::from_millis(100), "validation took {best:?}");
}
