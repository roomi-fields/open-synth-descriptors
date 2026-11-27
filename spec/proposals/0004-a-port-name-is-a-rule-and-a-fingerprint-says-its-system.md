# 0004 — A port name is a rule, and a fingerprint says the system it was taken on

## Status

- State: `draft`
- Candidate since: —
- Under the trial: —

Amended on 2026-11-25 with the precisions the building of 0.7 brought to the rules on port names (sections 1, 4 and 5, "What a 0.6 descriptor becomes" and the cases). This change of a `candidate` text puts it back to `draft` ([PROCESS.md](../PROCESS.md), section 2): the delay of 14 days starts again the day it enters `candidate` again.

This draft is amended with the owner's decisions on its three questions (section "Objections"); the content of 0.7 stays the owner's. The names of the fields and the examples below are the way the draft writes the decisions. The form of the two MIDI 2.0 signs (section 5) is likewise decided by the owner (2026-11-24).

## What it changes

The `identity.ports` field of the device schema, and two optional fields of the fingerprint schema, in 0.7; and the two signs of MIDI 2.0 that identify a device, the endpoint name and the product instance id (section 5): the first as an optional rule on the identity of a device descriptor and an optional text on a fingerprint, the second as an optional text on a fingerprint only.

## Why

What this brings to the sharing of sounds on devices: a device learnt on one system is recognised on another, so what is shared once serves everyone.

What is wrong without it. The same device has different port names by system: Windows gives "midiin2 electra controller", ALSA gives "Electra Controller Electra Port". Today a device descriptor lists its port names as plain strings (`schemas/0.6/device.schema.json:1472-1478`: "Its port names, for a device that does not answer"), and nothing says how a name is compared with what a machine shows, nor on which system it holds. A fingerprint holds the names one machine saw (`schemas/0.6/fingerprint.schema.json:37-60`), and does not say which system that was.

A first draft stored the names of each system exactly, in a block per system of the fingerprint. It was stopped: it kept a photograph of what one machine saw, not a rule of recognition. Real software does not store photographs; it states rules.

## What real software does

The sources of Cubase, Bitwig, DrivenByMoss, KnobKraft and the Web MIDI API were read by the maintainer on 2026-11-23, and are cited with their addresses as he read them. The MIDI 2.0 documents, MIDNAM and OSCQuery are read at their source in the section "What a reference does".

- **Steinberg Cubase, MIDI Remote API** (steinbergmedia.github.io/midiremote_api_doc, pages "Device Detection" and "New in v1.3"). A driver declares detection units. A unit holds predicates on the name of the input port and of the output port (equals, contains, starts with, ends with), may expect the reply to the identity request (manufacturer, family, model), and since API 1.3 may hold a platform filter (macOS, not macOS, Windows, not Windows). A real driver (Akai MPK249) pairs an almost empty name predicate with the identity reply: the identity decides.
- **Bitwig Studio, extension API 18** (maven.bitwig.com, `extension-api-18-sources.jar`, `ControllerExtensionDefinition.java` and `PlatformType.java`). A Java extension lists its port names per platform (WINDOWS, LINUX, MAC). A USB matcher exists beside the names; the reply to the identity request is deprecated there. The open set of scripts DrivenByMoss (github.com/git-moss/DrivenByMoss, `DefaultControllerDefinition.java:148-210`) generates the variants of a name by brute force: a prefix "N- " for N from 1 to 19 on Windows for a second identical device, the suffixes " [hw:N,0,port]" and " MIDI N" on Linux; a macOS port name appears in German ("Anschluss 1").
- **KnobKraft Orm** (`docs/programming-guide.md:168-240`). A probe message and a check of the reply per device, a scan of every port and of the 16 channels, a wait of 200 ms by default.
- **No tool scores**: each matches exactly or by pattern. None says what happens when two descriptors match, nor for two identical devices.
- **The Web MIDI API** gives a port's name, manufacturer and version only. In Chromium the version is the driver's, and on Windows the manufacturer is the driver's provider (`media/midi/midi_manager_*.cc`).

## The change

### 1 A port name in a descriptor is a rule

In the identity block of a device descriptor, an entry of `identity.ports` becomes an object, checked by the schema, with:

- `operator`: one of `equals`, `contains`, `starts_with`, `ends_with` (the four operators of Cubase's predicates);
- `value`: the text compared;
- `direction`: optional, `input` or `output`; when absent, the rule holds for both directions, and a name of either direction is enough (section 4);
- `platform`: optional, one of `windows`, `linux`, `macos`; when present, the rule holds for ports named on that platform only; when absent, it holds everywhere;
- `stack`: optional, one value of the closed list of section 3; when present, the rule holds for ports named by that stack only.

No pattern language lives in a string: the operator is a field, so one field has one meaning.

**A `stack` requires a `platform` and belongs to it.** A rule with a `stack` and no `platform` is refused, at `/stack`; a rule whose `stack` belongs to another platform than its `platform` (`windows` with `alsa-seq`) is refused, at `/stack`.

**What a plain name means today.** 0.6 says only this of the field: the type is a list of strings, described as "Its port names, for a device that does not answer" (`schemas/0.6/device.schema.json:1472-1478`); `spec/0.6.md` names port names among what the identity block holds (line 133) and gives no comparison rule. The one reader known to use `identity.ports` today, the detection of osc-bridge, reads a name as "the port name holds it, any case": its function `port_matches` carries the comment "A port name holds one of patterns, any case". Decided: a plain name of 0.6 is read as `contains`, case ignored, on both directions (no direction, no platform, no stack). The converter 0.6 → 0.7 writes it as a `contains` rule with no platform and no stack, inventing nothing else. Reason: compatibility is the first criterion of the domain; `equals` would stop recognising descriptors already published, which that detection recognises by a part of the name.

**Case.** Every operator ignores case: the comparison applies Unicode simple case folding, without normalisation, to both the `value` of the rule and the name of the fingerprint at the moment of comparison. A fingerprint's lower-case names and a rule's mixed-case value therefore compare equal. Reason: a port name's case varies by system for the same port (in Bitwig's own scripts, bitwig/bitwig-extensions, AkaiApcMiniDefinition, "APC Mini mk2 Control" on Linux and "APC mini mk2 Control" on Mac); a rule that kept case would need two rules for one port. The names of a fingerprint stay "lower case, words only" as in 0.6 (`schemas/0.6/fingerprint.schema.json:37-60`): 0.7 changes nothing there.

**An empty value matches nothing and is refused.** A rule whose `value` is empty would hold for every name under `contains`, `starts_with` and `ends_with`, and would say nothing under `equals`; the schema refuses it, at `/value`. This is the reading of osc-bridge's maintainer for the names of 0.6 too (2026-11-24): an empty pattern matches nothing. An empty port name of 0.6 therefore recognises nothing: the converter drops it, so no recognition changes and every valid 0.6 descriptor converts; a list that held no other name stays, empty. In memory, where a reader gives the names of an older text as rules, an empty value never matches.

**Evidence for `contains`: what a machine adds to a port name.** A rule aims at the word of the port, without the decorations a system puts around it, and `contains` allows that. Two kinds of evidence are kept apart, and none is a guarantee: a system may change its decorations.

- *Read in a source.* The Web MIDI API gives the name of a port as the system relays it (see "What real software does"). DrivenByMoss's generated variants show a prefix "N- " on Windows for a second identical device, the suffixes " [hw:N,0,port]" and " MIDI N" on Linux, and a macOS port name in German ("Anschluss 1"), all at `DefaultControllerDefinition.java:148-210`. Cubase's Launchkey MK3 driver shows 'MIDIIN' / 'MIDIOUT' and 'Port 2' in the name of one device by Windows stack (section 3).
- *An observation, not a rule of the system.* The maintainer of osc-bridge reports (2026-11-24) that ALSA appends a numeric `client:port` ("20:0") and may add an instance number in parentheses to the client ("Minilab3 (7)"); that Windows (WinMM) truncates names to 31 characters and prefixes a second unit of the same device ("2- ..."); that JACK through a2j prefixes "a2j: "; and that a device with several ports tells them apart by a word or a number ("MIDI 1", "DIN THRU", "MCU/HUI"). The documentation of ALSA, WinMM and JACK is not read here; these decorations rest on that report alone, as the observation of one maintainer of the one known detection.

Example:

```json
"ports": [
  { "operator": "contains", "value": "Electra Controller", "direction": "input" },
  { "operator": "starts_with", "value": "electra controller", "direction": "output", "platform": "linux", "stack": "alsa-seq" }
]
```

### 2 A fingerprint says the system it was taken on

A fingerprint gets two optional fields: `platform`, the platform the observation was taken on (`windows`, `linux`, `macos`), and `stack`, the naming stack, when the tool knows it (a value of the closed list of section 3). Their absence means "unknown", so a 0.6 fingerprint stays valid as written, and the converter writes nothing for it. The same refusals hold as for a rule: a `stack` without `platform`, and a `stack` of another platform than the `platform`, are refused at `/stack`. The names of a fingerprint (`ports`) stay as they are. They are facts of observation, not a block of names.

`via` is unchanged and says the channel (a browser or a native program); `platform` and `stack` say the system. A fingerprint taken in a browser carries the names the browser relays from the host, so its `platform` is the host's, when the tool knows it; a browser rarely knows the stack, and then says none.

### 3 The closed list of platforms and of stacks

The grain is both: the platform, and under it, optionally, the naming stack. Reason: the owner's intention is that the more a descriptor can say, the better a device is recognised, while a browser or a program that knows only its platform is still served. The platform is the grain of Cubase's platform filter and of Bitwig's `PlatformType`; the stack is the grain of the systems' own interfaces, and two stacks of one platform can name one port differently. A real Cubase driver (Novation Launchkey MK3 49, three detection units) names the same device two ways on Windows: under the comment "Windows" the port name contains 'LKMK3 MIDI' and 'MIDIIN' / 'MIDIOUT'; under "Windows RT" it contains 'LKMK3 MIDI' and 'Port 2'; on Mac it equals 'Launchkey MK3 49 LKMK3 DAW Out' / 'In' (read by the maintainer on 2026-11-23). A rule that names the stack says which naming it holds for.

The platforms: `windows`, `linux`, `macos`.

The stacks, each with its platform and the system's own documentation as its source (cited by address; this session did not read them again):

| Stack | Platform | What it is | Source |
|---|---|---|---|
| `winmm` | `windows` | the Windows multimedia MIDI functions (`midiInGetDevCaps`, `midiOutGetDevCaps`) | Microsoft Learn, "MIDI Reference" (learn.microsoft.com/windows/win32/multimedia/midi-reference) |
| `windows-midi-services` | `windows` | Windows MIDI Services, the MIDI 2.0 stack | Microsoft, Windows MIDI Services documentation (microsoft.github.io/MIDI) |
| `winrt` | `windows` | the Windows Runtime MIDI API (`Windows.Devices.Midi`), which Cubase's driver calls "Windows RT" | Microsoft Learn, "Windows.Devices.Midi Namespace" (learn.microsoft.com/uwp/api/windows.devices.midi) |
| `alsa-seq` | `linux` | the ALSA sequencer | ALSA project, library reference "MIDI Sequencer" (alsa-project.org/alsa-doc/alsa-lib/seq.html) |
| `alsa-rawmidi` | `linux` | the ALSA raw MIDI interface | ALSA project, library reference "RawMidi Interface" (alsa-project.org/alsa-doc/alsa-lib/rawmidi.html) |
| `jack` | `linux` | the JACK audio connection kit, its MIDI ports | JACK API documentation, `jack/midiport.h` (jackaudio.org/api) |
| `coremidi` | `macos` | Core MIDI | Apple, "Core MIDI" (developer.apple.com/documentation/coremidi) |

The lists are closed, as any closed list: a platform or a stack outside them enters by a new version, and is refused until then.

### 4 How a rule is matched against a fingerprint

A rule is applied to a fingerprint, or not applied: "not applied" is not an error, the rule simply says nothing about that fingerprint. Four statements decide whether a rule holds for the system of a fingerprint:

1. A rule with no `platform` applies to every fingerprint, including one with no `platform`.
2. A rule that names a `platform` and no `stack` applies only to a fingerprint that names the same platform, whatever its stack, or none.
3. A rule that names a `platform` and a `stack` applies only to a fingerprint that names the same platform and the same stack.
4. A fingerprint with no `platform`, or with a `platform` and no `stack`, does not match a rule that names what it lacks: the rule is not applied.

So the four cases of a rule against a fingerprint, with or without a stack on each side, give:

| Rule | Fingerprint | Result |
|---|---|---|
| a platform, no stack | a platform, no stack, same platform | applied |
| a platform, no stack | a platform and a stack, same platform | applied |
| a platform and a stack | a platform and the same stack | applied |
| a platform and a stack | a platform, no stack, same platform | not applied |

The names decide too. A rule is not applied to a fingerprint that has no list of names for the rule's `direction`, or, for a rule without `direction`, no list of names at all.

Once a rule is applied, its operator compares its `value` with the fingerprint's names of the rule's `direction`, or, without `direction`, with the names of every list present, with case ignored as in section 1. One name that suits it is enough: the rule is matched. A list that is present with no name that suits it, empty or not, gives not matched.

### 5 Two signs of MIDI 2.0 beside the port names

MIDI 2.0 gives a device two signs of identity that a port name does not (read at the source, section "What a reference does"): the name the device gives its own UMP endpoint, and the product instance id. The maintainer of osc-bridge, whose detection is the one known, reads both and says the format lacks them (2026-11-24). The draft writes them as follows.

- **The endpoint name.** On the identity of a device descriptor, an optional `identity.endpoint_name`, a rule with the same four operators as a port rule (`equals`, `contains`, `starts_with`, `ends_with`), a `value`, case ignored as for port names (section 1), an empty value refused. It has no `direction`: an endpoint is one for both directions, where a MIDI 1.0 port is one per direction. It has no `platform` and no `stack`: the name is the one the device gives, not one a system's naming stack builds. On a fingerprint, an optional `endpoint_name`, the text observed, lower case as the port names are.
- **The product instance id.** On a fingerprint only, an optional `product_instance_id`, the text observed, compared exactly (an identifier is not a name: no case folding, no operator). It does not belong to a device descriptor: it tells two units of one device apart, so it cannot be a property of a model.

A rule on the endpoint name is applied to a fingerprint that carries an `endpoint_name`, and says nothing about one that has none, as for the other rules (section 4). A 0.6 descriptor or fingerprint carries neither sign, so the converter 0.6 → 0.7 writes nothing for them.

The comparison of the product instance ids of two fingerprints says only whether the two ids are equal: the same, different, or unknown when one of the two fingerprints holds none or an empty one. It reads no other field and never says that two fingerprints are one unit: the detection engine decides that, from the `device`, the `usb` identity and the identity reply. What the ids do not settle of "two identical devices" stays in "What it leaves out".

Decided by the owner (2026-11-24): the identity block gains both, as written, because the one known detection of the reference reader reads both signs already, and because a device that answers over MIDI 2.0 is then recognised by what it says of itself, not only by what a system calls its ports.

### 6 Port names stay the weakest sign

A rule on a port name is the sign for a device that does not answer: it is the weakest, since a system builds the name and changes it (section 1). The order in which a detection reads its signs is not a rule of the format. It is the ranking of the detection engine, as its levels of trust are: the maintainer of osc-bridge ranks the identity reply first, then the USB identity, then the port name (2026-11-24). The sources show a part of it: Cubase's MPK249 driver pairs an almost empty name predicate with the identity reply, which then decides. The proposal says nothing of the order, and changes nothing of the other signs (`identity_reply`, `usb`, `sends`).

### What a 0.6 descriptor becomes

Read as it stands in 0.7 after its converter: a plain name becomes a `contains` rule with no platform and no stack, and an empty name is dropped (section 1); a fingerprint keeps its fields, with no `platform`, no `stack`, no `endpoint_name` and no `product_instance_id`. Nothing is invented.

**A resolution that joins versions.** Rule 23.22 of the text, which joins two versions in the more recent one, extends to profiles: when a device, its `midi_of` target or one of its profiles is of 0.7, each older text is converted to 0.7 before any join, the `midi_of` links first, then the profiles, and the result states 0.7.

## What it leaves out

The condition that this draft wait for what a base of device identities needs is lifted. Its reason: the maintainer of the one known detection, osc-bridge, answered on 2026-11-24 with the signs it reads (the identity reply, the USB identifiers, the port name, the nature of the client), the two MIDI 2.0 signs the format lacks (section 5) and the decorations of port names (section 1). The needs of that detection are therefore known. The base itself stays the catalogue's matter, not the format's.

- a score across several signs;
- the ranking when two descriptors match, and the order in which a detection reads its signs (section 6);
- two identical devices: the format compares two product instance ids (section 5) and judges no unit; a device that gives no instance id, or an id that is not unique or not stable, stays unsettled; the prefixes of DrivenByMoss show that the problem is real;
- a base of identities.

These are the matter of the detection software and of the catalogue.

## The conformance cases it would need

Not written here.

| Case | Expected |
|---|---|
| a rule with each operator and each direction | accepted |
| a rule with a `platform` | accepted |
| a rule with a `platform` and a `stack` of that platform | accepted |
| a plain name of 0.6, converted | a `contains` rule with no platform and no stack, nothing else added |
| an empty port name of 0.6, converted | dropped; a list of empty names stays, empty |
| an empty port name of 0.6, read without conversion, against a fingerprint | never matched |
| a rule without `direction`, against a fingerprint with a suiting name in either list | matched |
| a rule without `direction`, against a fingerprint with no list of names | not applied |
| a rule with a `direction`, against a fingerprint with no list for that direction | not applied |
| a rule against a present list with no name that suits it | not matched |
| two empty product instance ids, or one empty, compared | unknown |
| a 0.6 device under a 0.7 profile, or a 0.7 device with a 0.6 profile, resolved | each older text converted to 0.7 before any join; the result states 0.7 |
| an operator outside the list | refused at that operator |
| a rule with a `stack` and no `platform` | refused at `/stack` |
| a rule with `platform` `windows` and `stack` `alsa-seq` | refused at `/stack` |
| a `platform` or a `stack` outside the lists | refused at that field |
| a fingerprint with a `platform`, or with a `platform` and a `stack` | accepted |
| a fingerprint without `platform` | accepted: platform unknown |
| a fingerprint with a `stack` and no `platform` | refused at `/stack` |
| a fingerprint with `platform` `windows` and `stack` `alsa-seq` | refused at `/stack` |
| a rule with no platform, against a fingerprint with none | applied |
| a rule with a platform, against a fingerprint of the same platform with no stack | applied |
| a rule with a platform, against a fingerprint of another platform | not applied |
| a rule with a platform and a stack, against a fingerprint of the same platform and stack | applied |
| a rule with a platform and a stack, against a fingerprint of the same platform with no stack | not applied |
| a rule with a platform, against a fingerprint with no platform | not applied |
| a rule with a platform and a stack, against a fingerprint of the same platform and another stack | not applied |
| a rule with the value "Electra Controller", against a fingerprint with the name "electra controller" | applied: case ignored |
| a rule with an empty `value` | refused at `/value` |
| an `identity.endpoint_name` with each operator | accepted |
| an `identity.endpoint_name` with a `direction`, a `platform` or a `stack` | refused at that field |
| a fingerprint with an `endpoint_name` and a `product_instance_id` | accepted |
| a device descriptor with a `product_instance_id` in its identity | refused at that field |
| a rule on the endpoint name, against a fingerprint with none | not applied |
| a product instance id compared with the same text in another case | different: compared exactly |
| a 0.6 descriptor or fingerprint, converted | neither `endpoint_name` nor `product_instance_id` written |

The cases of the suite that prove these precisions: under `conformance/0.7/match/`, `rule-without-direction-takes-a-name-of-either-list`, `rule-without-direction-with-one-list-present`, `fingerprint-without-names-is-not-applied`, `empty-list-of-names-is-not-matched` and `empty-name-of-0-6-is-never-matched`; under `conformance/0.7/convert/`, `empty-port-name-is-removed`, `profile-empty-port-name-is-removed` and `only-empty-port-names-leave-an-empty-list`; under `conformance/0.7/same_product_instance_id/`, `same-text-is-same`, `other-case-is-different`, `empty-on-one-side-is-unknown` and `empty-on-both-sides-is-unknown`; under `conformance/0.7/resolve/`, `profile-0-6-port-names-under-a-0-7-device`, `profile-0-7-under-a-0-6-device`, `profile-0-6-extending-a-0-7-profile` and `identity-from-a-0-7-profile-programs-from-a-0-6-one`.

## What a reference does

- **Cubase and Bitwig**: see "What real software does". OSD takes the four operators, the direction, and the platform; it departs from Bitwig's lists of names per platform, which repeat a name where one rule would say it.
- **MIDNAM** (`MIDINameDocument10.dtd`, the element `MasterDeviceNames`). A document names its device by a `Manufacturer` and one or more `Model` elements, or by a `Device` element with a `Name` and a `UniqueID`; it holds no port name and no rule. 0004 follows it in naming a device by what the device is, not by a system's words. It departs from it because MIDNAM says which device a document is for, not how a machine recognises that device: the port-name rule, the platform and the stack have no place in it, and this proposal is about recognition.
- **MIDI 2.0, UMP endpoint discovery** (M2-104-UM, UMP Format & MIDI 2.0 Protocol v1.1.2, section 7.1: 7.1.1 Endpoint Discovery, 7.1.3 Device Identity Notification, 7.1.4 Endpoint Name Notification, 7.1.5 Product Instance Id Notification). A UMP endpoint answers a discovery request with, among others, its name (UTF-8, at most 98 bytes) and a product instance id (ASCII text, 32 to 126, at most 42 bytes), which the specification says may re-identify a device after a power cycle, distinguish several devices of one model, and unify several endpoints under one device; it should be, where possible, the serial number. 0004 follows it: it carries the endpoint name and the product instance id (section 5), with the specification's own words, and the instance id on a fingerprint only, since it names a unit and not a model.
- **MIDI 2.0, MIDI-CI discovery** (M2-101-UM, MIDI-CI v1.2, section 4.1 The First MIDI-CI Transaction: Discovery, sections 5.5 and 5.6; section 5.8.3.1 for the endpoint's product instance id, limited there to 16 bytes). A device answers a discovery message with its manufacturer, family, model and version identifiers. This is the identity the format already carries as `identity_reply`; 0004 changes nothing there.
- **MIDI 2.0, Property Exchange DeviceInfo** (M2-105-UM, MIDI-CI PE Foundational Resources v1.1.1, section 3, Table 5). A device that implements Property Exchange shall return, as `manufacturerId`, `familyId`, `modelId`, `versionId` with their names `manufacturer`, `family`, `model`, `version`, and may return a `serialNumber`. It carries the same data as the identity reply, in the same form. 0004 adds nothing from it: the identity is the one the format already has, and the serial number is the product instance id, written once (section 5).
- **OSCQuery** (the proposal's `HOST_INFO`). The host may give a human-readable `NAME`, which it is not required to give; it carries no manufacturer, model or version. 0004 follows it in treating a name as optional text, and adds nothing: OSCQuery says what an address space holds, not which device it is.

Where 0004 departs from MIDI 2.0 as a whole: those signs carry an identity the device answers with (manufacturer, family, model, version; endpoint name; product instance id). 0004 keeps the port names, which a device does not answer and a system builds, as the weakest sign for a device that does not answer, so that a device that only speaks MIDI 1.0 is still recognised. The reason is the domain's first criterion, compatibility: descriptors already published list port names, and a device that answers neither the identity request nor MIDI 2.0 discovery has only its port names to be recognised by. Where it follows them: a device that answers is described by what it says of itself (identity reply, endpoint name, instance id); which sign a detection reads first stays its engine's choice (section 6).

## Objections

The three questions of the draft are answered by the owner's decisions of 2026-11-24, given as an intention (recognise devices better: the more a descriptor can say, the better) and settled by his principles (a universal language, a facilitator for everyone, compatibility first). None remains open; each decision is written into the section where it decides.

The owner's last point is answered (2026-11-24): the identity block gains the endpoint name (a rule) and the fingerprint the product instance id, with the words of the MIDI 2.0 specification (section 5). None remains open.

- **1. The meaning of a plain name of 0.6.** Decided: it reads as `contains`, case ignored, on both directions. Reason: compatibility is the first criterion of the domain; the one known reader (the detection of osc-bridge) already reads it so, and `equals` would stop recognising descriptors already published. Written in section 1.
- **2. The grain of the system.** Decided: both. A rule may name the platform (`windows`, `linux`, `macos`) and, optionally, the naming stack under it; a fingerprint says its platform and, when it knows it, its stack. A rule that names a stack applies only where the stack is known and the same; a rule that names only the platform applies to every stack of it. Reason: more information recognises better, while a browser or a program that knows only its platform is still served. The stacks are a closed list grown by version. Written in sections 1 to 4.
- **3. Whether every operator ignores case.** Decided: every operator ignores case. Reason: a port name's case varies by system for the same port; a rule that kept case would need two rules for one port. Whether the names of a fingerprint stay "lower case, words only": they stay as in 0.6, 0.7 changes nothing. Written in section 1.
