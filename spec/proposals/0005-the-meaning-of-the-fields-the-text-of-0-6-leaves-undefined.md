# 0005 — The meaning of the fields the text of 0.6 leaves undefined

## Status

- State: `candidate`
- Candidate since: 2026-11-24
- Under the trial: —

This proposal decides nothing until it is accepted. The owner has read the first draft (2026-11-24) and decided its rule: the definition of a field is the intention it was created with, not the plain sense of its name and not what one descriptor happens to hold. This draft applies that rule. The first use of the process of proposals on a published version.

## What it changes

The text of the patch version 0.6.1 (`spec/0.6.1.md`) and the descriptions of its schemas (`schemas/0.6.1/`): descriptions only, no field, no value and no rule changes, and every 0.6 descriptor stays valid (`spec/ERRATA.md` 2.6 and 2.7).

## Why

This follows the cycle of proposals because it states a meaning that the text of 0.6 did not give, which is a change of the text of 0.6.1 and not a restatement of it.

What this brings to the sharing of sounds on devices: a third party who reads the text of OSD alone knows what every field is, so that the format can be implemented from its text. What is wrong without it: the schemas of 0.6 hold 248 properties with no description; 158 of them restate what the text already says, and are described in the patch version without a proposal. This proposal takes the other 90: the meaning of 88 of them is not in the text, and the two others are the boolean schema `false`, which forbids a key (below). The text of 0.6 even says (Annex A, "Taken out of 0.6") that `first_pc`, `first_number`, `rate_limit_hz` and `wrap_sysex` are "each defined in this text"; none is.

## The count and the list

The list of the properties without a description in `schemas/0.6` was replayed with the command `osd-reference undescribed schemas/0.6` of the reference part: 248 lines, identical to the committed list (`reference/undescribed-0.6.txt`), each pointer once. The 90 pointers of this proposal are among them (none is absent, none twice); the 158 others restate the text.

| What the 90 are | Count |
|---|---|
| Defined below (section "The definitions") | 83 |
| Left to the owner (section "Questions for the owner") | 5 |
| The boolean schema `false`, defining nothing | 2 |

The two schemas `false` are `#/$defs/resource/$defs/file_write_step/else/properties/chunk_bytes` and `#/$defs/resource/$defs/file_write_step/else/properties/coding` (`schemas/0.6/device.schema.json:514`: `"else": { "properties": { "chunk_bytes": false, "coding": false } }`): the `else` of a step that is not a write forbids the two keys. They hold no description and this proposal defines nothing for them (the meaning of `chunk_bytes` and `coding` is described where they are properties, `device.schema.json:508-509`). A search of every value `false` or `true` given as a schema in the six schemas of 0.6 finds these two only (the other `false` are values of `additionalProperties`, `uniqueItems` and `default`).

## What a proof is

The rule of the owner: a field is what it was created as. The kinds of evidence, in this order:

- **intention d'origine** (original intention): words written when the field was created, quoted verbatim. Three sources: **A**, osc-bridge, `docs/DEVICE_JSON_SCHEMA.md` (2026-05-14 to 2026-11-16), where the format was born; **B**, the schemas of OSD 0.1 (`schemas/0.1`, commit 3983e0b), also an older example when the field did not change; **C**, SyntHub, preset model and public API documentation, where sounds and firmwares were born. A, B and C were read at their source by the supervisor, who gave the words; this session did not read A or C itself, and cites them as given.
- **lecteur**: what the reader does or says, as `file:line` of `osd/src` (or of `osd-formats/src`, which reads and writes descriptors too);
- **cas**: a conformance case of `conformance/`;
- **exemple**: a descriptor of `examples/`, or the `examples` of a schema;
- **sens du nom**: the ordinary sense of the name, last, said as such, used only where the name, the type of the schema and at least one example or case describe the same thing without ambiguity.

Every definition is one sentence, says what the field is, and was set against each occurrence of the field in the 0.6 descriptors of the repository (examples and cases): none contradicts it. Where an original intention does contradict a 0.6 descriptor, the definition leaves the contradicted detail out and the contradiction is listed in the tensions. Where no original words were found, the line says "no original words found". A definition about a transport is worded from the side of the software that is reached and names no reader.

## The definitions

### Device: arguments, commands, triggers

- `#/$defs/arg/properties/default` — the value the argument takes when a message gives none, of the type of the argument. Original: A, `{"name":"mode","type":"enum","values":{"user":0,"arturia":1,"daw":2},"default":"user"}`; lecteur `osd/src/device.rs:1288-1289`.
- `#/$defs/arg/properties/type` — the kind of value the argument carries, one of `u7` (0..127, one byte), `u14` (0..16383), `s32` (signed), `int`, `float`, `bool`, `string`, `enum`. Original: A, the table "Arg types" (u7 0..127 one byte; u14 0..16383; s32 its range, signed; enum "see values: integer after lookup"; string; bool 0/1); lecteur `osd/src/device.rs:1274-1277, 1285`.
- `#/$defs/arg/properties/values` — the named values of an `enum` argument, each label with its integer. Original: A, `"values":{"user":0,"arturia":1,"daw":2}`, "integer after lookup"; lecteur `osd/src/device.rs:1264, 1175`, cas `conformance/0.6/read/values-given`.
- `#/$defs/command/properties/args` — the ordered list of the arguments the command takes, each named and typed. Original: A, `"args": [ // ordered list of OSC arguments`; lecteur `osd/src/device.rs:1406`, cas `conformance/0.6/read/args-forms`.
- `#/$defs/command/properties/forward` — the OSC message the command sends in place of a frame when the software is reached by OSC, with its `path` and its `args` (typed literals or `{name}`). Original: A, "When transport.kind = osc, commands use forward instead of frame", `"path": "/live/song/set/tempo", // OSC path emitted to target`, `"args": ["{bpm}"] // typed literals or {name}`; cas `conformance/0.6/read/args-forms/descriptor.json:5`.
- `#/$defs/command/properties/pre/items/properties/bind` — the name of an intermediate value that the `expr` beside it computes from the arguments, by simple arithmetic, before the message is built. Original: A, `"pre": [ // optional — derive intermediate vars` `{"bind":"pad_id","expr":"pad + 4"} // simple arithmetic on bound args`; cas `conformance/0.5/convert/pre-tree/descriptor.json:4-6`, lecteur `osd/src/device.rs:1393-1397`.
- `#/$defs/command/properties/section` — the part of the device the command belongs to (for example "General", "Filter"), as the dataset the entry came from names it. Original: A, example only (`"section": "General"`), no sentence; it is the "section" column of the MIDI Guide dataset the entries were imported from. No example of a command in the 0.6 descriptors contradicts it.
- `#/$defs/cc_entry/properties/section` — the part of the device the control belongs to, as above. Original: A, as above.
- `#/$defs/param_entry/properties/section` — the part of the device the parameter belongs to, as above. Original: A, as above.
- `#/$defs/trigger/properties/section` — the part of the device the trigger is listed under (`"Drums"`, `"General"`). Original: A, `"section": "General"`, "Drivers imported from MIDI Guide carry the device's drum map here, adapted from its <Device>.triggers.csv"; exemple `examples/0.5/synth.device.json:97`, lecteur `osd-formats/docs/midi-triggers-csv.md:9`.
- `#/$defs/trigger/properties/sound` — the name of the sound the trigger fires. Original: A, "A triggers block names each sound", `"sound": "Kick"`; lecteur `osd/src/device.rs:1422`.
- `#/$defs/trigger/properties/note` — the note the sound is played on, the entry's natural note when it is not given. Original: A, "note defaults to the entry's natural note", `"note": 36`; exemple `examples/0.5/synth.device.json:103`, lecteur `osd-formats/docs/midi-triggers-csv.md:15`.
- `#/$defs/trigger/properties/velocity_usage` — what the velocity of the note does to the sound, as a free text. Original: A, `"velocity_usage": "1~127: Accent level"`; exemple `osd-formats/tests/it/midi_triggers_csv_import.rs:31, 55` (`"Soft: damped; hard: open"`).
- `#/$defs/trigger/properties/comments` — a free remark about the trigger, written for people. No original words found; exemple `osd-formats/tests/it/midi_triggers_csv_import.rs:30, 54` (`"Choke group 1"`), lecteur `osd-formats/docs/midi-triggers-csv.md:16`.
- `#/$defs/reply/properties/match_args` — the arguments of the OSC message that the reply is matched on, each named and typed, so that the message maps back to the device's named surface. Original: A, "When the target pushes an OSC message back ..., the replies table maps it back into the device's named surface": `"match_osc": "/live/song/get/tempo", "match_args": [{"name": "bpm", "type": "float"}], "emit_osc": "/transport/tempo {bpm}"`; lecteur `osd/src/device.rs:1437-1440, 1449`.
- `#/$defs/reply/properties/match_args/items/properties/name` — the name the value of the argument takes in the emitted message (`{bpm}`). Original: A, as above.
- `#/$defs/reply/properties/match_args/items/properties/type` — the kind of the received argument: `int`, `float`, `string` or `bool`. Original: A, "int, float, string, bool".

### Device: the software and how it is reached

- `#/$defs/device/properties/device_id` — the product bytes that follow the manufacturer ID in the device's SysEx header, 0 to n bytes. Original: A, `"device_id": [0x7F,0x42], // product bytes following MID (0..n bytes)`, beside `"manufacturer_id":[0x00,0x20,0x6B], // SysEx MID (may be 1 or 3 bytes)` and `"sysex": {"header": [0xF0, 0x00, 0x20, 0x6B, 0x7F, 0x42]}`; the 0.4 and 0.5 examples hold `[]` (`examples/0.4/test_synth.json:18`), which is 0 bytes. It is not the unit number of MIDI; see the tensions.
- `#/$defs/device/properties/transport/properties/kind` — the way the software is reached, `midi` (the default) or `osc`; it decides how the commands of the device are sent. Original: A, `"kind": "osc", // "midi" (default) or "osc"`, "transport.kind is what flips the runtime branch. When absent, the driver behaves as a classical MIDI driver"; lecteur `osd/src/device.rs:1001, 1003`, cas `conformance/0.6/read/rate-limit-hz/descriptor.json:7`.
- `#/$defs/device/properties/transport/properties/host` — the address of the machine where the software listens for OSC. Original: A, `"host": "127.0.0.1", "port": 11000, // bridge → target`, said from the software's side here.
- `#/$defs/device/properties/transport/properties/port` — the port the software listens on for OSC. Original: A, as above.
- `#/$defs/device/properties/transport/properties/reply_port` — the port the software sends its OSC messages to when it answers or pushes a change. Original: A, `"reply_port": 11001 // target → bridge`, "When the target pushes an OSC message back to the bridge's reply_port", said from the software's side here.
- `#/$defs/device/properties/transport/properties/passthrough_prefix` — the first segment of the OSC addresses of a software's own surface that is not modelled command by command: everything under `/<prefix>/...` goes to it as it is, and what it sends back carries the prefix again. Original: A, "passthrough mode for environments with a user-defined OSC surface: forward everything under /<prefix>/... verbatim, no per-command modelling" (osc-bridge, CHANGELOG); "The driver just declares transport.passthrough_prefix and the bridge forwards anything under /<env>/... verbatim. Replies travel the other way with the prefix added back." (osc-bridge, README).
- `#/$defs/device/properties/transport/properties/rate_limit_hz` — the most messages a second sent to the software through this transport; the same field as `rate_limit_hz` of the device, which the schema describes. Original: A, `"rate_limit_hz": 800 // optional MIDI-out throttle`; cas `conformance/0.6/read/rate-limit-hz/descriptor.json:7` and its note ("given on the device and, for a transport, on that transport").
- `#/properties/subscriptions/items/properties/on` — when the one-shot message of the subscription is sent; `startup` is the only moment of 0.6. Original: A, "One-shot OSC messages emitted to the target at startup, typically to subscribe to remote state changes. Only on: startup is supported today."; lecteur `osd/src/device.rs:212-219`, cas `conformance/0.5/convert/address-segments/descriptor.json:11`.
- `#/properties/custom_commands/items/properties/wrap_sysex` — when true, the bytes the script returns are wrapped in `F0 … F7`. No original words found; cas `conformance/0.6/read/custom-command-wrap-sysex-and-profile` (its note).

### Device: identity

- `#/$defs/identity/properties/device_id/properties/min` — the lowest unit number the device can be set to. Original: A, `"device_id": { "default": 0x10, "min": 0x10, "max": 0x1F }, // the byte {dev} stands for`; Roland profile `{ "default": 16, "min": 16, "max": 31 }`.
- `#/$defs/identity/properties/device_id/properties/max` — the highest unit number the device can be set to. Original: A, as above.
- `#/$defs/identity/properties/family` — the bytes of the model, as its identity reply carries them, that `{family}` stands for in a pattern of the maker. Original: A, `"family": [0x59, 0x01], // the model's bytes, for {family} / {member}`, "the maker's pattern, the model's bytes"; lecteur `osd/src/frame.rs:404-410`, `spec/0.6.md:353`, cas `conformance/0.4/resolve/profile-of-a-family`.
- `#/$defs/identity/properties/member` — the bytes of the model within its family, that `{member}` stands for. Original: A, as above.
- `#/$defs/identity/properties/reply_text` — a text that the identity reply must also hold. Original: A, `"reply_text": "FM-1_9", // text the reply must also hold`; the values of the cases (`"acme"`, `"p6"`, `"keys"`) are texts.
- `#/$defs/identity/properties/sends` — what the device sends by itself, unasked, which tells models apart; the same block as `sends` of a fingerprint. No original words found; lecteur `osd/src/device.rs:1539`, exemple `examples/0.6/minilogue.fingerprint.json:25-28`.
- `#/$defs/identity/properties/firmware/properties/at` — the byte offsets, counted from the `F0` of the reply, that `decode` reads to give the firmware version. Original: A, "firmware.at gives byte offsets from F0; decode reads them"; the number of offsets A gives per `decode` (`ascii` 1, `lua` none) is left out: see the tensions.
- `#/$defs/identity/properties/firmware/allOf/0/if/properties/decode` — the case where `decode` is `json`, which requires `field`. No original words found; `spec/0.6.md:198`, cas `conformance/0.6/read/capabilities-decode-lua-missing-refused` (the twin case for `lua`).
- `#/$defs/identity/properties/firmware/allOf/1/if/properties/decode` — the case where `decode` is `lua`, which requires `script`. No original words found; `spec/0.6.md:200`, cas as above.
- `#/$defs/usb_id/properties/vendor_id` — the USB vendor id of the device, 0 to 65535. No original words found; cas `conformance/0.6/read/device-identity-without-midi-first/descriptor.json:4`, exemple `examples/0.6/minilogue.fingerprint.json:15`; sens du nom.
- `#/$defs/usb_id/properties/product_id` — the USB product id of the device, 0 to 65535. As `vendor_id`.

### Device: controls and roles

- `#/if/anyOf/0/properties/cc_params` — the test, in the condition that makes `role_vocabulary` required, of the block `cc_params`. No original words found; `spec/0.6.md:127`, cas `conformance/0.6/read/role-without-vocabulary-refused`.
- `#/if/anyOf/0/properties/cc_params/properties/entries` — the test that one entry of `cc_params` carries a `role`; then `role_vocabulary` is required. As above.
- `#/if/anyOf/1/properties/params` — the same test for the block `params`. As above.
- `#/if/anyOf/1/properties/params/properties/entries` — the test that one entry of `params` carries a `role`. As above.
- `#/properties/params/properties/value_type` — the encoding of the value of the parameters of the block when it is set: `u7`, `u14` or `s32`. Original: A, `"value_type": "u14", // for SET value encoding: u7, u14 or s32`; lecteur `osd/src/device.rs:104, 115`, cas `conformance/0.5/convert/frame-params/descriptor.json:6`.

### Device: programs and banks

- `#/properties/programs/properties/banks` — the banks of programs of the device. Original: A, "A bank sends CC 0 when it has msb, CC 32 when it has lsb (none, one or both), then the program change first_pc + the program's place in the bank."; lecteur `osd/src/device.rs:1079, 1106`.
- `#/$defs/bank/properties/name` — the name of the bank. No original words found; cas `conformance/0.6/read/bank-first-pc-and-first-number/descriptor.json:6`, lecteur `osd-formats/src/midnam.rs:115-116, 161`.
- `#/$defs/bank/properties/msb` — the value of the Bank Select MSB (control 0) that selects the bank, sent when the bank has one. Original: A, "A bank sends CC 0 when it has msb"; lecteur `osd-formats/src/midnam.rs:100, 163`.
- `#/$defs/bank/properties/lsb` — the value of the Bank Select LSB (control 32) that selects the bank, sent when the bank has one. Original: A, "CC 32 when it has lsb (none, one or both)"; lecteur `osd-formats/src/midnam.rs:101, 166`.
- `#/$defs/bank/properties/first_pc` — the Program Change number of the first program of the bank; the program change sent for a program is `first_pc` plus its place in the bank. Original: A, "then the program change first_pc + the program's place in the bank"; cas `conformance/0.6/read/bank-first-pc-and-first-number`.
- `#/$defs/bank/properties/size` — the number of programs of the bank. Original: A, "size programs"; lecteur `osd-formats/src/midnam.rs:168`.
- `#/$defs/bank/properties/first_number` — the number the device shows for the first program of the bank, 1 when it is not given. Original: A, "numbered from first_number (1 by default) as the device shows them"; cas as above.
- `#/properties/programs/properties/program_change` — when false, no program change is sent to recall a program: `before` and `after` do the recall on their own (a SysEx scene change, Song Select). Original: A, "With \"program_change\": false no program change is sent: before and after recall on their own (a SysEx scene change, Song Select)"; cas `conformance/0.6/read/programs-after/descriptor.json:8`.
- `#/properties/programs/properties/notes` — a free text about the programs, for people. Original: A, `"notes": "free text"`.

### Device: where the data comes from

- `#/$defs/source/properties/url` — the address of the source. No original words found; lecteur `osd-formats/src/credit.rs:6-8`, cas `conformance/0.6/read/resource-deluge-definition/descriptor.json:16`.
- `#/$defs/source/properties/author` — who made the source. No original words found; lecteur `osd-formats/src/credit.rs:6-8` ("who made it").
- `#/$defs/source/properties/attribution` — the credit text the source asks to be kept whenever its data is reused. Original: C, `"attribution": "MIDI Guide (https://midi.guide) by Pencil Research and contributors"`, "Every such file records the licence, the attribution, the upstream CSV and the commit it came from" (SyntHub blog), "keep the attribution when you reuse them" (SyntHub API documentation, about CC BY-SA 4.0 data); no sentence defines the field, this is its only use.
- `#/$defs/source/properties/commit` — the commit of the repository at `url` that the data was taken at. No original words found (the SyntHub blog names "the commit it came from"); cas `conformance/0.6/read/resource-deluge-definition/descriptor.json:16-18`.
- `#/$defs/source/properties/imported_at` — the date the data was imported from the source. No original words found; lecteur `osd-formats/src/credit.rs:24`, `osd-formats/docs/electra-one.md:40`.
- `#/$defs/source/properties/notes` — a free text about the source, for people. No original words found; exemple `examples/0.6/minilogue.device.json:7`, cas `conformance/0.6/read/resource-deluge-definition/descriptor.json:20`.

### Fingerprint

- `#/properties/maker_replies/items/properties/request` — a read-only request of the maker that was sent to the device, in hex, as `F0 … F7`. No original words found; lecteur `osd/src/fingerprint.rs:28, 58`, exemple `examples/0.6/minilogue.fingerprint.json:21`.
- `#/properties/names/items/properties/name` — the name the person gave to the control, at most 60 characters. No original words found; lecteur `osd/src/fingerprint.rs:151-155`, and the description of `names` of the schema.
- `#/properties/sends` — what the device sent by itself, unasked, while it was observed. No original words found; lecteur `osd/src/device.rs:1539`, exemple `examples/0.6/minilogue.fingerprint.json:25-28`.

### Firmware

- `#/properties/name` — the name of the firmware. No original words found; exemple `examples/0.6/tribe-0.3-beta.firmware.json:3`, lecteur `osd/src/firmware.rs:21-25`; sens du nom. A firmware is "a firmware, mod or script a device can run: who makes it, where it lives, its licence, what it changes to the device's MIDI, and how it is installed. It points to the author's own release; it never carries the firmware itself." (B, `schemas/0.1/firmware.schema.json`).
- `#/properties/homepage` — the address of the web page where the firmware lives. Original: B, "where it lives" (the sentence above); exemple `examples/0.6/tribe-0.3-beta.firmware.json:27`, lecteur `osd/src/firmware.rs:40`.
- `#/properties/install` — how the firmware is installed: a method, its steps, and how to go back to the maker's firmware. Original: B, "how it is installed", and for `revert` "How to go back to the maker's firmware." (`schemas/0.1/firmware.schema.json:204`); lecteur `osd/src/firmware.rs:253-254`, cas `conformance/0.6/read/firmware-builder`.

### Sound

- `#/properties/category` — the musical category of the sound (bass, lead, pad…), one of the ways a preset is described, which its author may change with a new version. Original: C, "How a preset is described; its author may change it with a new version"; exemple `examples/0.6/warm-pad.sound.json:11` (`"pad"`).
- `#/properties/genres` — the musical genres of the sound, described in the same way. Original: C, as above; exemple `examples/0.6/warm-pad.sound.json:12`.
- `#/properties/key` — the musical key of the sound. Original: C, `Musical key, e.g. "A minor"`.
- `#/properties/version` — the number of the version of the sound, from 1; its author changes its description with a new version. Original: C, "its author may change it with a new version"; B, example 0.1 `"version": 1`; lecteur `osd/src/sound.rs:100`.
- `#/properties/published` — the date and time the sound was shared. Original: C, SyntHub writes it from the date the sound was shared; B, example 0.1 `"published": "2026-10-27T12:00:00Z"`.
- `#/properties/updated` — the date and time of the last version of the sound. Original: C, SyntHub writes it from the date of the sound's last version.
- `#/properties/origin/properties/source` — the site the sound was first published on, as a short name (for example `patchstorage`). Original: C, "Where an imported preset was published first": source "The site it comes from, e.g. \"patchstorage\""; and B, "Where it was first published, when not here." for `origin`.
- `#/properties/origin/properties/url` — the page of the sound on that site. Original: C, url "Its page there"; lecteur `osd/src/sound.rs:333-336` (`url` is required there).
- `#/properties/origin/properties/author` — the name of the author of the sound on that site. Original: C, author "Its author's name there".
- `#/properties/origin/properties/author_url` — the page of the author on that site, where a claim's code is looked for. Original: C, author_url "The author's page there, where a claim's code is looked for".
- `#/properties/links` — the named links about the sound, each with a name and an address. No sentence in C; its use: SyntHub writes one link, `{ "name": "SyntHub", "url": "<page of the sound>" }`; lecteur `osd/src/sound.rs:343` ("A named link").
- `#/properties/links/items/properties/name` — the name of the link, as it is shown. As above.
- `#/properties/links/items/properties/url` — the address of the link. As above.
- `#/properties/preview` — what lets one hear and see the sound: an audio file, an image. No original words found; lecteur `osd/src/sound.rs:351-353`.
- `#/properties/preview/properties/audio` — an audio file that lets one hear the sound. As above.
- `#/properties/preview/properties/image` — an image of the sound (a picture of its settings, a cover). As above.
- `#/properties/preview/properties/audio/oneOf/0/properties/url`, `#/properties/preview/properties/image/oneOf/0/properties/url` — the address the file can be fetched from. No original words found; the description of the same object in `content.sysex` of the schema ("A file, beside the descriptor or online"), lecteur `osd/src/sound.rs:353`.
- `#/properties/preview/properties/audio/oneOf/0/properties/sha256`, `#/properties/preview/properties/image/oneOf/0/properties/sha256` — the SHA-256 of the bytes of the file, as 64 lower-case hexadecimal digits. No original words found; lecteur `osd/src/sound.rs:281-291`, `spec/0.6.md` 10.3.

## Questions for the owner

Fields with no evidence of any kind: no original words, no case, no example, and a reader that only carries the value. What was searched for each: the reader (`osd/src`, `osd-formats/src`), the text `spec/0.6.md`, the schemas, `examples/` and `conformance/` of every version, the docs of `osd-formats`; the birth documents were searched by the supervisor (A, B, C).

1. `#/$defs/command/properties/notes` — no original words; no example or case; the schema has `_`-keys for remarks, whether `notes` is another thing is not shown.
2. `firmware #/properties/description`, `sound #/properties/description` — no original words (C gives "How a preset is described" for the group, not this field); no example or case carries one.
3. `firmware #/properties/release/properties/date` — no original words (B says only "Where this version is downloaded, from its author."); no example; the date could be of the release or of the download.
4. `sound #/properties/tags` — no original words (the same group sentence of C), no example or case (unlike `genres`); whether tags differ from genres is not shown.

The count: 5 pointers, in 4 questions: device 1, firmware 2 (the description and `release.date`), sound 2 (the description and `tags`).

## Tensions with the intentions of OSD, for a later version

The definitions above state what each field was created as; a patch version changes no field. Each original intention was set against the intentions of OSD (one meaning per field; names reused from MIDI, MIDNAM and OSCQuery; no reader privileged). Where they conflict, the definition keeps the original and the conflict is listed here for a later version:

1. `device.device_id` is named like MIDI's "device ID" (the unit number, which `identity.device_id` carries) but holds the product bytes after the manufacturer ID: one name, two meanings in one descriptor, against "one meaning per field" and "names reused from MIDI"; its bytes are also in `sysex.header`.
2. `transport.host`, `port`, `reply_port` and `passthrough_prefix` were written from the standpoint of one reader ("bridge → target"); the definitions say them from the software's side (where it listens, where it sends its replies) and name no reader. `passthrough_prefix` describes a behaviour of a reader rather than a fact of the software.
3. `sound.origin.source` holds a site key of one catalogue (`patchstorage`), with no list of values; `origin.author_url` was created for one catalogue's claim mechanism: the definition keeps "the author's page on that site".
4. `section` (command, control, parameter, trigger) has no definition of its own and no list of values: it came with an import of one dataset.
5. `identity.firmware.at`: A gives the number of offsets per `decode` (`ascii` 1, `lua` none), but 0.6 cases write two offsets with `ascii` (`conformance/0.5/resolve/profile-own-keys-win/case.json:10`, `"at": [12, 15]`) and one with `lua` (`conformance/0.6/read/capabilities-lua-fields/descriptor.json:21`); the definition says no count.
6. `transport.rate_limit_hz` was created as a "MIDI-out throttle" but the field also sits on an OSC transport; the definition says "sent to the software through this transport".
7. `command.forward` and `reply.match_args` were created for OSC only, in a block (`command`) that has `frame` for MIDI: one block holds two ways of being reached, the way is chosen by `transport.kind` outside the block.
8. `sound.published` and `sound.updated` are written by one catalogue from its own dates; the format does not say they are the author's.

## What a reference does

The references were read at their source by the supervisor only for the birth documents of OSD (A, B, C above). The others:

- MIDI 2.0 Property Exchange, DeviceInfo: not read. Property Exchange is named by the descriptor for `priority` (the schema), not here.
- MIDNAM: not read at its DTD. `osd-formats` reads and writes it, and its table (`osd-formats/docs/midnam.md:50-60, 109-110`) shows the correspondence of a bank with `PatchBank`, of `msb` and `lsb` with the controls 0 and 32, and of `first_pc` with `Patch@ProgramChange`: this proposal quotes that table and not the DTD.
- OSCQuery: not read.
- The pencilresearch trigger table (`template.triggers.csv`): not read; `osd-formats/docs/midi-triggers-csv.md` gives its columns as the maintainers read them.

## Objections

None yet.

Decided by the owner (2026-11-24): the 5 pointers of the section "Questions for the owner" stay without a description in 0.6.1, since no evidence of any kind gives their meaning; they and the tensions listed above are examined again for a later version.
