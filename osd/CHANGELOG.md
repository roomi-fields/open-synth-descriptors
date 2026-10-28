# Changelog

## Unreleased

- Reads OSD 0.4 strictly: device, maker profile, sound, fingerprint and firmware descriptors into their types, documentary fields included, and writes them back unchanged.
- Resolves a driver's shared MIDI (`midi_of`) and maker profiles through a `Files` source, as osc-bridge 0.16 does; refusals are typed faults with their file and JSON pointer.
