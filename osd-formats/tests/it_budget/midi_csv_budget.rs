//! Docs/CADRE.md R17: translating one device of about 200 rows takes under 10 ms. A time
//! budget: played at rest, by name; a failure under load is to be replayed.

use crate::common::{csv, resolved, row, source};
use osd_formats::{export, import, Format};
use std::time::{Duration, Instant};

#[test]
fn importing_and_exporting_a_device_of_200_rows_each_take_under_10_ms() {
    let rows: Vec<Vec<String>> = (0..200)
        .map(|n| {
            row(&[
                ("manufacturer", "Lofty"),
                ("device", "Trundler"),
                ("section", "Oscillators"),
                ("parameter_name", &format!("Parameter {n}")),
                ("cc_msb", &format!("{}", n % 128)),
                ("cc_min_value", "0"),
                ("cc_max_value", "127"),
            ])
        })
        .collect();
    let text = csv(&rows);
    let src = source();
    let mut best_import = Duration::MAX;
    let mut best_export = Duration::MAX;
    for _ in 0..5 {
        let start = Instant::now();
        let imported = import(&text, Format::MidiCsv, &src).expect("the file imports");
        best_import = best_import.min(start.elapsed());
        let device = resolved(imported.value);
        let start = Instant::now();
        export(&device, Format::MidiCsv).expect("the device exports");
        best_export = best_export.min(start.elapsed());
    }
    assert!(best_import < Duration::from_millis(10), "import took {best_import:?}");
    assert!(best_export < Duration::from_millis(10), "export took {best_export:?}");
}
