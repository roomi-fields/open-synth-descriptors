# 0004 — A port name is a rule, and a fingerprint says the system it was taken on

## Status

- State: `draft`
- Candidate since: —
- Under the trial: —

This draft decides nothing: the content of 0.7 is the owner's. Every field name, every value and every example below is a way to write the idea, to be settled by the owner, not a decision.

## What it changes

The `identity.ports` field of the device schema, and an optional field of the fingerprint schema, in 0.7.

## Why

What this brings to the sharing of sounds on devices: a device learnt on one system is recognised on another, so what is shared once serves everyone.

What is wrong without it. The same device has different port names by system: Windows gives "midiin2 electra controller", ALSA gives "Electra Controller Electra Port". Today a device descriptor lists its port names as plain strings (`schemas/0.6/device.schema.json:1472-1478`: "Its port names, for a device that does not answer"), and nothing says how a name is compared with what a machine shows, nor on which system it holds. A fingerprint holds the names one machine saw (`schemas/0.6/fingerprint.schema.json:37-60`), and does not say which system that was.

A first draft stored the names of each system exactly, in a block per system of the fingerprint. It was stopped: it kept a photograph of what one machine saw, not a rule of recognition. Real software does not store photographs; it states rules.

## What real software does

Each source was read by the maintainer on 2026-11-23; this session did not read them again, so the facts are cited with their addresses as the maintainer read them.

- **Steinberg Cubase, MIDI Remote API** (steinbergmedia.github.io/midiremote_api_doc, pages "Device Detection" and "New in v1.3"). A driver declares detection units. A unit holds predicates on the name of the input port and of the output port (equals, contains, starts with, ends with), may expect the reply to the identity request (manufacturer, family, model), and since API 1.3 may hold a platform filter (macOS, not macOS, Windows, not Windows). A real driver (Akai MPK249) pairs an almost empty name predicate with the identity reply: the identity decides.
- **Bitwig Studio, extension API 18** (maven.bitwig.com, `extension-api-18-sources.jar`, `ControllerExtensionDefinition.java` and `PlatformType.java`). A Java extension lists its port names per platform (WINDOWS, LINUX, MAC). A USB matcher exists beside the names; the reply to the identity request is deprecated there. The open set of scripts DrivenByMoss (github.com/git-moss/DrivenByMoss, `DefaultControllerDefinition.java:148-210`) generates the variants of a name by brute force: a prefix "N- " for N from 1 to 19 on Windows for a second identical device, the suffixes " [hw:N,0,port]" and " MIDI N" on Linux; a macOS port name appears in German ("Anschluss 1").
- **Ableton Live control surfaces** (decompiled scripts, so labelled: not the vendor's documentation). USB vendor and product ids with the model name first, then the identity request sent twice and its reply compared.
- **KnobKraft Orm** (`docs/programming-guide.md:168-240`). A probe message and a check of the reply per device, a scan of every port and of the 16 channels, a wait of 200 ms by default.
- **No tool scores**: each matches exactly or by pattern. None says what happens when two descriptors match, nor for two identical devices.
- **The Web MIDI API** gives a port's name, manufacturer and version only. In Chromium the version is the driver's, and on Windows the manufacturer is the driver's provider (`media/midi/midi_manager_*.cc`).

## The change

### 1 A port name in a descriptor is a rule

In the identity block of a device descriptor, an entry of `identity.ports` becomes an object, checked by the schema, with:

- `operator`: one of `equals`, `contains`, `starts_with`, `ends_with` (the four operators of Cubase's predicates);
- `value`: the text compared;
- `direction`: `input` or `output`;
- `system`: optional, one value of the closed list of section 3; when present, the rule holds for ports named by that system only; when absent, it holds for every system.

No pattern language lives in a string: the operator is a field, so one field has one meaning.

**What a plain name means today.** 0.6 says only this of the field: the type is a list of strings, described as "Its port names, for a device that does not answer" (`schemas/0.6/device.schema.json:1472-1478`); `spec/0.6.md` names port names among what the identity block holds (line 133) and gives no comparison rule. The one reader known to use `identity.ports` today, the detection of osc-bridge, reads a name as "the port name holds it, any case": its function `port_matches` carries the comment "A port name holds one of patterns, any case". The draft therefore reads a plain name of 0.6 as `contains`, case ignored, on both directions (no direction, no system), and the owner is asked to confirm (open objection below). The converter to 0.7 writes it as such, inventing nothing else. The alternative is `equals`: its cost is that descriptors already published, which that detection recognises by a part of the name, would stop being recognised.

Example:

```json
"ports": [
  { "operator": "contains", "value": "electra controller", "direction": "input" },
  { "operator": "starts_with", "value": "electra controller", "direction": "output", "system": "linux" }
]
```

### 2 A fingerprint says the system it was taken on

A fingerprint gets one optional field, `system`: the system the observation was taken on, a value of the closed list below. Its absence means "unknown", so a 0.6 fingerprint stays valid as written, and the converter writes nothing for it. The names of a fingerprint (`ports`) stay as they are. It is one fact of observation, not a block of names.

`via` is unchanged and says the channel (a browser or a native program); `system` says the system. A fingerprint taken in a browser carries the names the browser relays from the host, so its `system` is the host's, when the tool knows it.

### 3 The closed list of systems, and its grain

Two grains are possible.

**Grain A, the platform** (the grain of Cubase's platform filter and of Bitwig's `PlatformType`): `windows`, `linux`, `macos`.

**Grain B, the naming stack**: `winmm` and `windows-midi-services` on Windows, `alsa-seq` and `alsa-rawmidi` on Linux, `coremidi` on macOS. Its reason: two stacks of one platform can name one port differently, and a rule could then say which stack it holds for. The cost: a reader must know which stack it runs on, which a browser or a program often does not.

Two stacks of one platform do name a port differently, and the rules answer it without a finer list. A real Cubase driver (Novation Launchkey MK3 49, three detection units) names the same device two ways on Windows: under the comment "Windows" the port name contains 'LKMK3 MIDI' and 'MIDIIN' / 'MIDIOUT'; under "Windows RT" it contains 'LKMK3 MIDI' and 'Port 2'; on Mac it equals 'Launchkey MK3 49 LKMK3 DAW Out' / 'In' (read by the maintainer on 2026-11-23). A descriptor writes two rules for the platform, one per naming, as that driver does with two units.

**Recommendation: grain A.** It is the grain that both mature references use; two stacks of one platform that name a port differently are answered by two rules for the platform, as the Cubase driver above does with two units; and a fingerprint taken by a browser or a program usually knows its platform and often not its stack. A value can later be added by a new version, as for any closed list. The alternative, grain B, stays open for the owner, with the cost above.

| Value (grain A) | Names |
|---|---|
| `windows` | the ports as Windows names them, whatever its MIDI stack |
| `linux` | the ports as Linux names them, whatever its MIDI stack |
| `macos` | the ports as macOS names them |

The list is closed: a system outside it enters by a new version.

### 4 Port names stay the weakest sign

The references order their signs the same way: the reply to the identity request and the USB identity first, the names last (Cubase's MPK249 driver lets the reply decide; Ableton starts from the USB ids). The proposal keeps that order: a rule on a port name is the last sign that a detection reads, and the proposal changes nothing of the other signs (`identity_reply`, `usb`, `sends`).

### What a 0.6 descriptor becomes

Read as it stands in 0.7 after its converter: a plain name becomes the rule of section 1; a fingerprint keeps its fields, with no `system`. Nothing is invented.

## What it leaves out

This draft is expected to be refined when a base of device identities exists; the base is the catalogue's matter, not the format's, and the draft does not pass to candidate before what the base needs of the format is known.

- a score across several signs;
- the ranking when two descriptors match;
- two identical devices (the prefixes of DrivenByMoss show that the problem is real);
- a base of identities.

These are the matter of the detection software and of the catalogue.

## The conformance cases it would need

Not written here.

| Case | Expected |
|---|---|
| a rule with each operator and each direction | accepted |
| a rule with a `system` | accepted |
| a plain name of 0.6, converted | the `contains` rule, case ignored, nothing else added |
| an operator outside the list | refused at that operator |
| a fingerprint with a `system` | accepted |
| a fingerprint without `system` | accepted: system unknown |
| a `system` outside the list | refused at `/system` |

## What a reference does

- **Cubase and Bitwig**: see "What real software does". OSD takes the four operators, the direction, and the platform; it departs from Bitwig's lists of names per platform, which repeat a name where one rule would say it.
- **MIDNAM, MIDI 2.0 Property Exchange, OSCQuery**: **not read** for this draft; none is known to this author to carry port-name rules.

## Objections

Open questions for the owner. None is decided; each carries the draft's recommendation and its evidence.

- **1. The meaning of a plain name of 0.6.** Recommendation: read it as `contains`, case ignored, on both directions. Evidence: the one known reader of `identity.ports`, the detection of osc-bridge, reads a name as "the port name holds it, any case" (its function `port_matches`); `schemas/0.6/device.schema.json:1472-1478` gives no rule. Alternative: `equals`, which would stop the recognition of descriptors already published. Open.
- **2. The grain of the list of systems.** Recommendation: the platform (`windows`, `linux`, `macos`). Evidence: Cubase's platform filter and Bitwig's `PlatformType` use that grain; a real Cubase driver (Novation Launchkey MK3 49) answers two stacks of one platform with two rules. Alternative: the naming stack, at the cost that a browser or a program often does not know its stack. Open.
- **3. Whether every operator ignores case.** Recommendation: every operator ignores case. Evidence: in Bitwig's own scripts (bitwig/bitwig-extensions, AkaiApcMiniDefinition) the Linux and the Mac names of one port differ only by a capital ("APC Mini mk2 Control" and "APC mini mk2 Control"); a rule that respected case would need two rules for one port. Alternative: each operator keeps case, the descriptor saying otherwise. Whether the names of a fingerprint stay "lower case, words only" as in 0.6 (`schemas/0.6/fingerprint.schema.json:37-60`) is not decided here either. Open.
