//! Reading and resolving the catalogue that `SYNTHHUB_DEVICES` names: each
//! driver under 50 µs per kB of the files it reads, the whole catalogue
//! under 2 s, on a machine at rest (osd/docs/CADRE.md R16). On a loaded
//! machine nothing is measured: its times say nothing of the reader.
//! The files are held in memory first, so only `osd` is measured.
//!
//! The catalogue is measured as stored, then converted to 0.7 in memory, the
//! conversion out of the time measured, under the same bounds, per kB of the
//! converted files read. A driver the conversion or the resolution refuses is
//! left aside there and counted: the catalogue test carries the refusal
//! (`cargo test -p osd`).
//!
//!     SYNTHHUB_DEVICES=<catalogue> cargo bench -p osd

use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Each driver is measured this many times; its time is the fastest, so a
/// pause of the machine is not counted as the reader's.
const RUNS: usize = 5;
/// A driver's budget per kB (1000 bytes) of the files it reads.
const PER_KB: Duration = Duration::from_micros(50);
/// The most load (the last minute's average, `/proc/loadavg`) a machine at
/// rest has.
const AT_REST: f64 = 1.0;

fn main() {
    let Some(dir) = std::env::var_os("SYNTHHUB_DEVICES") else {
        println!("SYNTHHUB_DEVICES is not set: nothing measured");
        return;
    };
    let load = std::fs::read_to_string("/proc/loadavg").ok().and_then(|l| l.split(' ').next()?.parse::<f64>().ok());
    if let Some(load) = load.filter(|l| *l >= AT_REST) {
        println!("the machine is loaded ({load}, at rest under {AT_REST}): nothing measured");
        return;
    }
    let mut files = HashMap::new();
    let mut drivers = Vec::new();
    for maker in std::fs::read_dir(&dir).unwrap() {
        let maker = maker.unwrap().path();
        if !maker.is_dir() {
            continue;
        }
        for f in std::fs::read_dir(&maker).unwrap() {
            let f = f.unwrap().path();
            if f.extension().is_none_or(|e| e != "json") {
                continue;
            }
            let id = format!("{}/{}", maker.file_name().unwrap().to_str().unwrap(), f.file_stem().unwrap().to_str().unwrap());
            if !f.file_stem().unwrap().to_str().unwrap().starts_with('_') {
                drivers.push(id.clone());
            }
            files.insert(id, std::fs::read_to_string(&f).unwrap());
        }
    }
    drivers.sort();

    println!("the catalogue as stored:");
    let mut over: Vec<String> = measure(&files, &drivers).into_iter().map(|why| format!("as stored: {why}")).collect();

    let converted = converted_to_0_7(&files);
    let kept: Vec<String> = drivers.iter().filter(|id| osd::resolve(id, &converted).is_ok()).cloned().collect();
    println!("the catalogue converted to 0.7:");
    println!("{} drivers left aside: the conversion or the resolution refuses them", drivers.len() - kept.len());
    over.extend(measure(&converted, &kept).into_iter().map(|why| format!("converted to 0.7: {why}")));

    assert!(over.is_empty(), "{}", over.join("\n"));
}

/// The drivers and maker profiles of `files` converted to 0.7, by the same
/// catalogue ids; a file whose conversion is refused is absent.
fn converted_to_0_7(files: &HashMap<String, String>) -> HashMap<String, String> {
    files
        .iter()
        .filter_map(|(id, text)| {
            let kind = match id.split_once('/')?.1 {
                name if name.starts_with("_profile") => osd::Kind::Profile,
                name if name.starts_with('_') => return None,
                _ => osd::Kind::Device,
            };
            Some((id.clone(), osd::convert_to(text, kind, osd::Version::V0_7).ok()?))
        })
        .collect()
}

/// Reads and resolves each of `drivers` from `files`, prints the times, and
/// returns each bound they exceed (osd/docs/CADRE.md R16).
fn measure(files: &HashMap<String, String>, drivers: &[String]) -> Vec<String> {
    if drivers.is_empty() {
        println!("no driver: nothing measured");
        return Vec::new();
    }
    let start = Instant::now();
    for id in drivers {
        osd::resolve(id, files).unwrap_or_else(|f| panic!("{f}"));
    }
    let whole = start.elapsed();

    let mut times: Vec<(Duration, &str, usize)> = drivers
        .iter()
        .map(|id| {
            let fastest = (0..RUNS)
                .map(|_| {
                    let t = Instant::now();
                    std::hint::black_box(osd::resolve(id, files).unwrap());
                    t.elapsed()
                })
                .min()
                .unwrap();
            let read: usize = osd::resolve(id, files).unwrap().files.iter().map(|f| files[f].len()).sum();
            (fastest, id.as_str(), read)
        })
        .collect();
    times.sort();
    let median = times[times.len() / 2].0;
    let mut rates: Vec<f64> = times.iter().map(|(d, _, size)| d.as_secs_f64() * 1e9 / *size as f64).collect();
    rates.sort_by(f64::total_cmp);
    println!("µs per kB: median {:.1}, p90 {:.1}, p99 {:.1}, max {:.1}", rates[rates.len() / 2], rates[rates.len() * 9 / 10], rates[rates.len() * 99 / 100], rates[rates.len() - 1]);
    let over: Vec<String> = times
        .iter()
        .filter(|(d, _, size)| d.as_nanos() * 1000 >= PER_KB.as_nanos() * *size as u128)
        .map(|(d, id, size)| format!("  {id} ({size} bytes): {d:?}"))
        .collect();
    println!("{} drivers read and resolved in {whole:?}; one driver: median {median:?}", drivers.len());
    println!("{} drivers take {PER_KB:?} per kB or more:\n{}", over.len(), over.join("\n"));
    let mut exceeded = Vec::new();
    if whole >= Duration::from_secs(2) {
        exceeded.push(format!("the catalogue takes {whole:?}"));
    }
    if !over.is_empty() {
        exceeded.push(format!("{} drivers take {PER_KB:?} per kB or more", over.len()));
    }
    exceeded
}
