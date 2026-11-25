# Changelog of the format

One entry per version of the format, the newest first: what the version changed, and what a descriptor of the version before becomes. The text of a version is in `spec/`; a published version never changes.

Each change names the cases of `conformance/<version>/convert` that prove it, or says "no conversion case" where it has none.

**What the reader reads.** The reader reads 0.4, 0.5, 0.6 and 0.7 (candidate), and converts from 0.4 to 0.5, then to 0.6, then to 0.7. 0.1, 0.2 and 0.3 are not read.

## 0.7 (candidate, not published)

In the public repository as a candidate while its proposals are candidates: it is not accepted and may change before its acceptance. Until the owner accepts them, its schema addresses and its book pages are not served. Source: `spec/0.7.md`, a copy of `spec/0.6.md`. 0.7 opens with its change of address alone.

- **Address**: the `$id` and `$schema` of each file name `0.7`; a 0.6 file takes the 0.7 `$schema` and nothing else changes; a 0.4 or 0.5 file reaches 0.7 through 0.6, but for a control that mixes kinds of message (below). Cases: `0-6-device-changes-only-its-schema`.
- **A control with no message** (proposal 0009): `ways` of an entry of `cc_params` or `params` is a list of zero or more ways. `ways: []` says the message is not known yet: such an entry claims no message and alone carries `range`, `transmit`, `recognize` and, in `cc_params`, `nrpn_lsb_range` (`UnknownField` beside a way); its reference scale is its `scale`, else its `range`. A reader warns of it, never refuses it. An entry of 0.4, 0.5 or 0.6 with no message (a `params` entry with a `range` and no number too) is read the same way, and the converter writes it with `ways: []`, these fields unchanged. Cases: `control-without-a-way-warned` and `params-entry-without-a-way-warned` (read), `transmit-beside-a-way-refused` (read), `entries-without-a-message-keep-their-fields`.
- **A control that mixes kinds of message** (proposal 0009): a control of `cc_params` of 0.4 or 0.5 whose flat fields mix a CC, an NRPN or an RPN, which 0.6 refuses, becomes one control with one way per message, in the order CC, NRPN, RPN, numbers kept, an NRPN or an RPN of 14 bits, `range` on every way and `nrpn_lsb_range` on the NRPN way. The converter refuses it when a field has no place in a way (an NRPN or an RPN given by one number, a `cc_lsb` without `cc`, an `nrpn_lsb_range` without an NRPN) and when one of its messages is claimed twice. A control of a fingerprint that mixes kinds stays refused, and so does the conversion to 0.6. Cases: `mixed-cc-and-nrpn-control-of-0-4-becomes-two-ways`, `mixed-cc-and-nrpn-control-of-0-5-becomes-two-ways`, `mixed-cc-and-rpn-control-becomes-two-ways`, `mixed-nrpn-and-rpn-control-becomes-two-ways`, `mixed-three-kinds-control-becomes-three-ways`, `mixed-control-with-nrpn-of-one-number-refused`, `mixed-control-with-cc-lsb-without-cc-refused`, `mixed-control-with-block-without-nrpn-refused`, `mixed-control-message-claimed-twice-refused`, `fingerprint-control-mixing-cc-and-nrpn-refused`, `fingerprint-name-control-mixing-cc-and-nrpn-refused`.
- **Title and description** (proposal 0007): a control, a `params` entry, a command and a trigger accept `title` (never empty) and `description`; a control accepts `value_description` and `notes`. The converter moves `_usage`, `_name`, `_description` and a control's `_notes` to them when the value is a string and the field is absent, and writes no `title` otherwise. Cases: `usage-becomes-value-description`, `usage-name-description-notes-become-fields`, `usage-without-values-still-becomes-value-description`, `usage-with-existing-value-description-keeps-both`, `name-with-existing-title-keeps-both`, `entry-without-name-gets-no-title`, `text-of-wrong-form-stays-under-its-underscore`, `trigger-description-becomes-description`.
- **Contexts** (proposal 0003): a device declares `contexts`; a way of `cc_params` carries `in` and `slice`; a message is claimed once by claims that can hold together, by the one rule of coordinates. The converter writes none (it invents no value). No conversion case; read cases under `conformance/0.7/read` (contexts, in, slice, midi-of-file-alone).
- **Who supplies a source** (proposal 0010): a source accepts `provider`, the identifier of an entry of the table of providers (`schemas/providers.json`, its own version, a date); a reader refuses a malformed one and warns of one its table does not hold. The four source types named after a product leave the list of `type`, which keeps its ten terms. The converter writes `contributed` for `electra-preset`, `pencilresearch`, `knobkraft` and `midnam`, with `provider` `electra-one`, `pencilresearch` and `knobkraft` (none for `midnam`) and `type in 0.6: <word>` as the first line of `notes`; a well-formed `provider` already there is kept, another value goes under `_provider`; the ten other terms are unchanged. Cases: `source-product-types-become-contributed`, `source-evidence-type-unchanged`, `source-provider-already-there`, `source-product-keys-become-general-fields`.
- **Port names and the signs of MIDI 2.0** (proposal 0004): an entry of `identity.ports` is a rule (`operator`, `value`, optional `direction`, `platform`, `stack`); `identity.endpoint_name` is a rule; a fingerprint says its `platform`, `stack`, `endpoint_name` and `product_instance_id`; the matching of a rule against a fingerprint and the comparison of two product instance ids are defined (5.3.7). The converter writes each plain name as a `contains` rule, drops an empty one, and writes neither sign. A resolution with a 0.7 profile or device converts the older texts before merging them (23.22). Cases: `port-name-becomes-contains-rule`, `profile-port-names-become-rules`, `empty-port-name-is-removed`, `profile-empty-port-name-is-removed`, `only-empty-port-names-leave-an-empty-list`, `ports-null-stays-null`, `fingerprint-changes-only-its-schema`.
- **`midi_of`**: a 0.6 file and its 0.7 target, or the reverse, are joined in 0.7 (rule 23.22). Cases: `midi-of-0-6-file-on-0-7-target`, `midi-of-0-7-file-on-0-6-target`.

## 0.6

Published 2026-11-17 as a draft, frozen with the checksums of its schemas in `edfdffe`. Source: `spec/0.6.md`, section 7 and Annex A. What a 0.5 file becomes is what the converter writes; a 0.5 file that uses nothing listed changes by its `$schema` alone. The cases are in `conformance/0.6/convert`.

- **A fingerprint names its subject** (`device` or `program`): a 0.5 fingerprint converts when its device holds its firmware, and is refused without a device or without the firmware. Cases: `fingerprint-named-device-converts`, `fingerprint-without-device-refused`, `fingerprint-device-without-firmware-refused`.
- **Firmware `targets`**: a 0.5 firmware becomes one target; its first-level `device`, `midi` and `based_on` are refused in 0.6. Cases: `firmware-device-becomes-target`, `firmware-0-4-device-becomes-target`, `firmware-midi-moves-into-the-target`.
- **An NRPN or RPN says its width** (`value_bits`): a 0.5 one becomes 14 bits. Case: `nrpn-rpn-value-gets-14-bits`.
- **One spelling**: a hex text is upper-case; a control never mixes a CC, an NRPN and an RPN; a sound holds at least one of `sysex`, `midi`, `settings`. Cases: `hex-lowercase-becomes-uppercase`, `control-mixing-cc-and-nrpn-refused`, `fingerprint-control-mixing-cc-and-nrpn-refused`, `fingerprint-name-control-mixing-cc-and-nrpn-refused`, `sound-without-content-refused`.
- **Address**: the `$id` and `$schema` of each file name `0.6`; a 0.5 file takes the 0.6 `$schema`. No conversion case.
- **`capabilities`** forbids a repeated entry; the converter writes it once. No conversion case.
- **Null keys** on `identity` and `programs` drop what the maker profile gives. No conversion case.
- **Roles**: a control may carry a `role`; a 0.5 file has none, none is added. No conversion case.
- **Controls**: `default`, `priority`, `step_count`, `transmit`, `recognize`, `values` of a CC, `also`; a 0.5 file gains none. No conversion case.
- **Banks and dumps**: `writable` on a bank, `reply_timeout_ms` on a dump. No conversion case.
- **Resources**: `resources` of a device (the Deluge file write). No conversion case.
- **Software devices**: `stands_for` and `fidelity`. No conversion case.
- **Sounds**: `external_ids`, `needs`, `made_on`, sound transfers (`sound_transfers`, `places`, `coding`); a 0.5 sound gains none. No conversion case.
- **Open lists**: any name of a resource, any non-empty `export`. No conversion case.
- **Sound `kind`**: an extension `x-<registrant>-<term>` beside the nine words. No conversion case.
- **Firmware kinds** `module` and `application`, and a `builder`. No conversion case.
- **Host device without MIDI**: a device with a USB identity and no MIDI of its own is valid. No conversion case.
- **Versions compared** follow one grammar. No conversion case.
- **Assembly**: a new document; a 0.5 file has none. No conversion case.
- **`midi_of`**: a file that follows a target adds only `params` entries. No conversion case.

Taken out of 0.6: nothing that 0.5 published.

## 0.5

Published 2026-10-30, with the checksums of its schemas in `814f9ac`. Source: `spec/0.6.md`, section 7 (and 8.3, 9.4). A converter converts every 0.4 descriptor to 0.5 as follows. The cases are in `conformance/0.5/convert`.

- **`$schema`**: the first key, an address under `https://w3id.org/osd/0.5/`, required. No conversion case.
- **Frame elements**: each `{placeholder}` string becomes a structure; `{checksum}` without an algorithm and `{function}` are refused. Cases: `frame-commands`, `frame-identity`, `frame-params`, `frame-profile`, `frame-programs-before`, `frame-programs-dump`, `frame-replies`, `fault-checksum-without-algorithm`, `fault-checksum-twos-without-7-bit-suffix`, `fault-function`.
- **`pre`**: an expression string becomes a tree of operations. Case: `pre-tree`.
- **Addresses**: a string with `{name}` becomes a list of segments; a malformed brace is refused. Cases: `address-segments`, `fault-brace-empty`, `fault-brace-unclosed`.
- **`forward.args` and `emit_osc`**: a `{name}` becomes an `arg` or `capture` object. Cases: `forward-args`, `emit-args`.
- **`midi_in`**: the object of templates becomes the list of its types; a non-canonical template is refused. Cases: `midi-in-list`, `midi-in-notes`, `midi-in-empty`, `fault-midi-template`.
- **Closed vocabularies**: `decode`, `expects`, `selects`, `_sources[].type` and `_sources[].license` take their terms from closed lists; a word outside the tables is refused. Cases: `vocabularies-merged`, `license-none`, `fault-selects-case`, `fault-encoding-lsb-msb`.
- **`capabilities`**: derived from the fields present, absent when empty. Case: `capabilities-lua`.

## 0.4

Published 2026-10-28; its checksums are recorded in `30e7200`. Source: the README's section Versions and `schemas/0.4`. There is no `conformance/0.4/convert`.

- The firmware rule of an `identity` may read a reply carrying JSON (`decode: json`, the version in `field`), as the Electra One answers. A 0.3 descriptor is not read; the reader reads 0.4 as it is. No conversion case.

## 0.3

Published 2026-10-27. Source: the README's section Versions and `schemas/0.3`. Not read.

- A fingerprint may name a device no descriptor describes yet (`device.vendor`, `device.model`) and the controls the person named while teaching it (`names`). No conversion case.

## 0.2

Published 2026-10-27. Source: the README's section Versions and `schemas/0.2`. Not read.

- The device's `identity` adds `usb` (USB vendor and product ids) and `sends` (active sensing, clock, how it ends a note). No conversion case.
- A new fingerprint descriptor. No conversion case.
- Every 0.1 device descriptor is a valid 0.2 one.

## 0.1

Published 2026-10-27. Source: the README's section Versions and `schemas/0.1`. Not read.

- Device, maker profile, sound, firmware. No conversion case.

## Tags

A tag is named `spec-<x.y.z>`, with `z` = 0 for a version `x.y`. Each is proposed on the commit that first recorded the checksums of the version's schemas, read from `git log`. This changelog sets no tag; the owner pushes them, after the switch, in the public repository.

| Tag | Commit |
|---|---|
| `spec-0.4.0` | `30e7200` |
| `spec-0.5.0` | `814f9ac` |
| `spec-0.6.0` | `edfdffe` |
