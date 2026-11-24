# Open Synth Descriptors — glossary

The words of the OSD format: what a descriptor says about a music device, its sounds and its firmwares. Each word has one meaning in the specification, the conformance cases and the `osd` reader.

## Devices and their description

**Device**:
The real machine, hardware or software, that is driven: a synthesizer, a drum machine, a controller, a piece of music software.
_Avoid_: instrument, machine, unit

**Driver**:
A device descriptor: an OSD file that describes one device, from one source and for one firmware.
_Avoid_: mapping, definition, device file

**Variant**:
One driver among several for the same device, told apart by its source and its firmware.
_Avoid_: version (kept for the format and the firmware)

**Maker profile**:
What a maker repeats across its devices (recognition, programs), described once and taken up by its drivers.
_Avoid_: template, base

**Shared MIDI**:
The link by which a driver takes another driver's MIDI and describes only what differs (`midi_of`).
_Avoid_: inheritance, parent

**Source**:
Where a driver comes from, and how far that origin is trusted: the maker's documentation, a community, an agent's contribution, a check on the hardware.
_Avoid_: tier, provenance

**Capability**:
What a driver requires from its reader beyond the declarative fields, such as a Lua script.
_Avoid_: extension, feature

**Catalogue**:
A published set of drivers and maker profiles.
_Avoid_: database, registry

**Fingerprint**:
What recognises a device: its identity reply, its port names, the controls it sends.
_Avoid_: signature; never said of a sound

## Sounds

**Sound**:
A complete setting of a device, taken from it, shared or imported, with its author and its licence.
_Avoid_: preset, patch

**Program**:
A memory slot of the device, recalled by its number and its bank.
_Avoid_: slot, preset

**Sound identity**:
What tells that two copies of a sound are the same sound.
_Avoid_: sound fingerprint

**Sound layout**:
Where each control of a device is stored in the bytes of a sound.
_Avoid_: sound format

## The format

**Version**:
A complete, immutable set of the format's schemas, named `<major>.<minor>`.
_Avoid_: release, edition

**Conformance case**:
An input (descriptors and their linked files) with the result every reader must return.
_Avoid_: fixture, example
