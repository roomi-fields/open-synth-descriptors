//! docs/CADRE.md R17: translating one preset of 200 controls takes under 10 ms. A time
//! budget: played at rest, by name; a failure under load is to be replayed.

use crate::common::electra::{control, cc7, preset, source};
use crate::common::resolved;
use osd_formats::{export, import, Format};
use std::time::{Duration, Instant};

#[test]
fn importing_and_exporting_a_preset_of_200_controls_each_take_under_10_ms() {
    let controls: Vec<_> = (0..200u64).map(|n| control(n + 1, &format!("Parameter {n}"), Some(1), cc7(n % 128), Some((0, 127)))).collect();
    let text = preset(&[(1, "Oscillators")], controls).to_string();
    let src = source();
    let mut best_import = Duration::MAX;
    let mut best_export = Duration::MAX;
    for _ in 0..5 {
        let start = Instant::now();
        let imported = import(&text, Format::ElectraOne, &src).expect("the preset imports");
        best_import = best_import.min(start.elapsed());
        let device = resolved(imported.value);
        let start = Instant::now();
        export(&device, Format::ElectraOne).expect("the device exports");
        best_export = best_export.min(start.elapsed());
    }
    assert!(best_import < Duration::from_millis(10), "import took {best_import:?}");
    assert!(best_export < Duration::from_millis(10), "export took {best_export:?}");
}
