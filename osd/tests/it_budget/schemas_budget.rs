//! Each text the reader converts of the SyntHub catalogue is valid against the schemas of the
//! version it declares, the published one, and the whole is judged in under 10 s. A time budget:
//! it is played by name, and a failure under load is to be replayed.

use crate::common::checks::{converted_catalogue, judge_converted, load_schemas};
use crate::common::{catalogue, report};
use std::time::{Duration, Instant};

#[test]
fn the_schemas_of_the_published_version_validate_the_converted_catalogue_in_under_10_s() {
    let Some(dir) = catalogue() else { return };
    // The files the conversion refuses are left out.
    let converted = converted_catalogue(&dir);
    assert!(converted.len() > 1000, "{} converted files", converted.len());
    load_schemas();
    let start = Instant::now();
    let failures = judge_converted(&converted);
    let took = start.elapsed();
    report("converted files not valid", failures);
    assert!(took < Duration::from_secs(10), "validation took {took:?}");
}
