//! The program `osd`.

fn main() {
    std::process::exit(osd_cli::run(std::env::args().skip(1).collect()));
}
