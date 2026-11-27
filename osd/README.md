# osd

The reference reader of [Open Synth Descriptors](https://github.com/roomi-fields/open-synth-descriptors) (OSD): an open format that describes synthesizers, drum machines, controllers and music software beyond what MIDI and OSC say.

`osd` reads a descriptor strictly into typed values, resolves a device's shared MIDI and maker profiles, reports its capabilities and converts a descriptor to a newer version of the format. Every refusal is a typed fault that names the file, the JSON pointer and the rule.

```toml
[dependencies]
osd = "0.6"
```

The command line is the `osd-cli` crate; the formats others use are in `osd-formats`: the pencilresearch/midi CSV, its triggers CSV, Electra One presets, MIDNAM, OSCQuery, the MIDI 2.0 controller list, the Deluge device definition and the KnobKraft Orm adaptation skeleton. The Deluge device definition and the KnobKraft Orm adaptation skeleton are exported only.

Licensed under MIT or Apache-2.0, at your option.
