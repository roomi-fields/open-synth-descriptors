# Extensions

The registry of the extensions of the closed vocabularies of 0.5, beside the schemas it extends. An
extension has the form `x-<registrant>-<term>` (`^x-[a-z0-9]+-[a-z0-9-]+$`); the rule, and what a
reader does with an unknown one, are in the [README](../README.md#extension). Listing here is
required to promote an extension to a core term in a later version.

Each entry: the extension, the field it extends, a one-sentence definition, its stable URL.

## `osd`: words of the SyntHub catalogue with no core term

The converter maps these 0.4 `selects` words to them.

| Extension | Field | Definition | URL |
|---|---|---|---|
| `x-osd-sampler-element` | `selects` | a recallable element of a sampler (a sample or a zone). | https://w3id.org/osd/extensions#x-osd-sampler-element |
| `x-osd-track` | `selects` | a stored track of a sequencer or a recorder. | https://w3id.org/osd/extensions#x-osd-track |
| `x-osd-canvas` | `selects` | a stored canvas of a visual or modular environment. | https://w3id.org/osd/extensions#x-osd-canvas |
| `x-osd-session` | `selects` | a stored session of a software host. | https://w3id.org/osd/extensions#x-osd-session |
| `x-osd-synth-slot` | `selects` | a slot that holds one synthesizer of a multi-engine device. | https://w3id.org/osd/extensions#x-osd-synth-slot |
| `x-osd-drum-slot` | `selects` | a slot that holds one drum sound of a multi-engine device. | https://w3id.org/osd/extensions#x-osd-drum-slot |
