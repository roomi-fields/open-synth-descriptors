//! The defined controllers of MIDI 1.0 (the table of Control Change messages of
//! the MIDI 1.0 Detailed Specification, controller number and name), the
//! reference a vocabulary term's `source` is checked against. A controller
//! of a pair (MSB 0-31, LSB 32-63) is listed by its MSB; the LSB is the same
//! name "(LSB)".

/// `(controller number, name)`.
pub const CONTROLLERS: &[(u8, &str)] = &[
    (0, "Bank Select"),
    (1, "Modulation Wheel"),
    (2, "Breath Controller"),
    (4, "Foot Controller"),
    (5, "Portamento Time"),
    (6, "Data Entry MSB"),
    (7, "Channel Volume"),
    (8, "Balance"),
    (10, "Pan"),
    (11, "Expression Controller"),
    (12, "Effect Control 1"),
    (13, "Effect Control 2"),
    (16, "General Purpose Controller 1"),
    (17, "General Purpose Controller 2"),
    (18, "General Purpose Controller 3"),
    (19, "General Purpose Controller 4"),
    (38, "Data Entry LSB"),
    (64, "Damper Pedal (Sustain)"),
    (65, "Portamento On/Off"),
    (66, "Sostenuto"),
    (67, "Soft Pedal"),
    (68, "Legato Footswitch"),
    (69, "Hold 2"),
    (70, "Sound Controller 1 (Sound Variation)"),
    (71, "Sound Controller 2 (Timbre/Harmonic Intensity)"),
    (72, "Sound Controller 3 (Release Time)"),
    (73, "Sound Controller 4 (Attack Time)"),
    (74, "Brightness"),
    (75, "Sound Controller 6"),
    (76, "Sound Controller 7"),
    (77, "Sound Controller 8"),
    (78, "Sound Controller 9"),
    (79, "Sound Controller 10"),
    (80, "General Purpose Controller 5"),
    (81, "General Purpose Controller 6"),
    (82, "General Purpose Controller 7"),
    (83, "General Purpose Controller 8"),
    (84, "Portamento Control"),
    (91, "Effects 1 Depth"),
    (92, "Effects 2 Depth"),
    (93, "Effects 3 Depth"),
    (94, "Effects 4 Depth"),
    (95, "Effects 5 Depth"),
    (96, "Data Increment"),
    (97, "Data Decrement"),
    (98, "Non-Registered Parameter Number LSB"),
    (99, "Non-Registered Parameter Number MSB"),
    (100, "Registered Parameter Number LSB"),
    (101, "Registered Parameter Number MSB"),
    (120, "All Sound Off"),
    (121, "Reset All Controllers"),
    (122, "Local Control"),
    (123, "All Notes Off"),
    (124, "Omni Mode Off"),
    (125, "Omni Mode On"),
    (126, "Mono Mode On"),
    (127, "Poly Mode On"),
];

/// Whether `(number, name)` is a row of the table.
pub fn is_defined(number: u64, name: &str) -> bool {
    CONTROLLERS.iter().any(|(n, w)| u64::from(*n) == number && *w == name)
}
