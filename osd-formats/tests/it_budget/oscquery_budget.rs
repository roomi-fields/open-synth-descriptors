//! Docs/CADRE.md R17: translating one device of about 200 controls to or from OSCQuery takes
//! under 10 ms. A time budget: played at rest, by name; a failure under load is to be replayed.

use crate::common::midnam::device_of_200_controls;
use crate::common::oscquery::source;
use osd_formats::{export, import, Format};
use std::time::{Duration, Instant};

#[test]
fn exporting_and_importing_a_device_of_200_controls_to_oscquery_each_take_under_10_ms() {
    let device = device_of_200_controls();
    let src = source();
    let (mut best_export, mut best_import) = (Duration::MAX, Duration::MAX);
    for _ in 0..5 {
        let start = Instant::now();
        let out = export(&device, Format::OscQuery).expect("the device exports");
        best_export = best_export.min(start.elapsed());
        let start = Instant::now();
        import(&out.value, Format::OscQuery, &src).expect("the namespace imports");
        best_import = best_import.min(start.elapsed());
    }
    assert!(best_export < Duration::from_millis(10), "export took {best_export:?}");
    assert!(best_import < Duration::from_millis(10), "import took {best_import:?}");
}
