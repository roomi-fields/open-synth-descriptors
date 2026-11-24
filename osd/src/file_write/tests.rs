//! The coding of the bytes of a chunk in 7 bits: `mcoded7` (the name of MIDI-CI Property Exchange,
//! M2-103-UM v1.2, 6.1.7: the sign bits of a group of seven bytes first, the first byte's sign bit
//! highest) and `msb_first_7in8` (the sign bit of byte k at bit k). Each case is the norm's own
//! example or its contrast.

use super::{encode, Coding};
use serde_json::Value;

fn bytes(hex: &str) -> Vec<u8> {
    hex.split_whitespace().map(|b| u8::from_str_radix(b, 16).unwrap()).collect()
}

#[test]
fn mcoded7_sends_a_group_of_seven_bytes_as_the_signs_then_the_seven_low_parts() {
    // AAAAaaaa … GGGGgggg with the signs 1 0 1 0 1 0 1 are sent as 0ABCDEFG = 0101 0101, then the low 7 bits.
    assert_eq!(encode(Coding::Mcoded7, &bytes("81 02 83 04 85 06 87")), bytes("55 01 02 03 04 05 06 07"));
}

#[test]
fn mcoded7_sends_a_short_group_with_the_signs_in_the_most_significant_bits_of_the_first_byte() {
    // AAAAaaaa BBBBbbbb CCCCcccc with the signs 1 0 1 are sent as 0ABC0000 = 0101 0000, then the low 7 bits.
    assert_eq!(encode(Coding::Mcoded7, &bytes("80 01 82")), bytes("50 00 01 02"));
}

#[test]
fn mcoded7_codes_complete_groups_then_the_bytes_left_over_as_a_short_group() {
    assert_eq!(encode(Coding::Mcoded7, &bytes("FF FF FF FF FF FF FF 80")), bytes("7F 7F 7F 7F 7F 7F 7F 7F 40 00"));
    assert_eq!(encode(Coding::Mcoded7, &[]), Vec::<u8>::new());
    assert_eq!(encode(Coding::Mcoded7, &bytes("00 7F")), bytes("00 00 7F"));
}

#[test]
fn msb_first_7in8_puts_the_sign_of_byte_k_at_bit_k_unlike_mcoded7() {
    let data = bytes("85 06 87 08 09 0A 8B");
    assert_eq!(encode(Coding::MsbFirst7in8, &data), bytes("45 05 06 07 08 09 0A 0B"));
    assert_eq!(encode(Coding::Mcoded7, &data), bytes("51 05 06 07 08 09 0A 0B"));
}

#[test]
fn msb_first_7in8_codes_a_short_last_group_with_its_signs_in_the_low_bits_of_the_first_byte() {
    assert_eq!(encode(Coding::MsbFirst7in8, &bytes("80 01")), bytes("01 00 01"));
}

#[test]
fn every_coded_byte_has_its_bit_7_clear() {
    let data: Vec<u8> = (0..=255u8).collect();
    for coding in [Coding::Mcoded7, Coding::MsbFirst7in8] {
        assert!(encode(coding, &data).iter().all(|b| b & 0x80 == 0));
    }
}

#[test]
fn the_coding_cases_of_the_conformance_suite_hold() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../conformance/0.6/coding");
    let mut seen = 0;
    for case in std::fs::read_dir(&root).unwrap_or_else(|e| panic!("{}: {e}", root.display())) {
        let path = case.unwrap().path().join("case.json");
        let c: Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        let coding = match c["coding"].as_str().unwrap() {
            "mcoded7" => Coding::Mcoded7,
            "msb_first_7in8" => Coding::MsbFirst7in8,
            other => panic!("{}: {other} is no coding", path.display()),
        };
        assert_eq!(encode(coding, &bytes(c["bytes"].as_str().unwrap())), bytes(c["coded"].as_str().unwrap()), "{}", path.display());
        seen += 1;
    }
    assert!(seen >= 6, "{seen} coding cases");
}
