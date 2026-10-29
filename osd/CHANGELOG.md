# Changelog

## Unreleased

- Reads OSD 0.4 strictly: device, maker profile, sound, fingerprint and firmware descriptors into their types, documentary fields included, and writes them back unchanged.
- Reads an integer by its value, as JSON Schema does: `1.0` and `1e2` are the integers 1 and 100, written back as `1` and `100`; a number with a fraction where an integer is expected stays refused.
- Resolves a driver's shared MIDI (`midi_of`) and maker profiles through a `Files` source, as osc-bridge 0.16 does; refusals are typed faults with their file and JSON pointer.
