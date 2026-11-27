# osd-formats

Open Synth Descriptors (OSD) to and from the formats others use. Imported and exported: the pencilresearch/midi CSV, its triggers CSV, Electra One presets, MIDNAM, OSCQuery and the MIDI 2.0 controller list. The Deluge device definition and the KnobKraft Orm adaptation skeleton are exported only. The crate builds on the reader [`osd`](https://crates.io/crates/osd) and says what a format cannot carry.

```toml
[dependencies]
osd-formats = "0.6"
```

See the [repository](https://github.com/roomi-fields/open-synth-descriptors) for the format.

Licensed under MIT or Apache-2.0, at your option.
