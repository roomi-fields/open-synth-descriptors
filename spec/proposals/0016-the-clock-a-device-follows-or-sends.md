# 0016 — The clock a device follows or sends

## Status

- State: `draft`
- Candidate since: —
- Under the trial: —

## What it changes

The text and the schemas of 0.8, the version that follows 0.7: the device descriptor gains an optional object, `sync`, at its root, with two lists of the MIDI clock messages, `follows` and `sends`.

## Why

What this brings to the sharing of sounds on devices: a device or a script that follows or sends the MIDI clock says so, so whoever shares a patch knows which end can set the pace and which can take it.

0.7 has no field of a device descriptor for this. `expects: clock` says what a control expects (section 6.4 of [0.7](../0.7.md)) and names no message. `sends.clock` belongs to a fingerprint and says a habit observed on one device. `midi_in` has no word for a clock message (section 9.3).

## The change

### The object

`sync` is a key of the root of a device descriptor, beside `midi_in` and `midi_out`. The block `device` keeps what names the device. It is written in the same way for hardware and for software.

It has two keys, `follows` and `sends`, each optional, and no other, but for notes (keys that start with `_`). Each is a list of words from a closed list of five. No extension `x-` is admitted, as in `midi_in`: MIDI 1.0 fixes these messages. A list names no word twice. The order of its words says nothing.

```json
{
  "properties": {
    "sync": {
      "type": "object",
      "additionalProperties": false,
      "patternProperties": { "^_": true },
      "properties": {
        "follows": { "$ref": "#/$defs/sync_messages" },
        "sends": { "$ref": "#/$defs/sync_messages" }
      }
    }
  },
  "$defs": {
    "sync_messages": {
      "type": "array",
      "uniqueItems": true,
      "items": {
        "enum": ["timing_clock", "start", "continue", "stop", "song_position_pointer"]
      }
    }
  }
}
```

The fragment shows what is added to the device schema. The property joins those of the root, and the definition joins its `$defs`.

### The five words

| Word | Message of MIDI 1.0 | Status byte | Family |
|---|---|---|---|
| `timing_clock` | Timing Clock | F8 | System Real Time |
| `start` | Start | FA | System Real Time |
| `continue` | Continue | FB | System Real Time |
| `stop` | Stop | FC | System Real Time |
| `song_position_pointer` | Song Position Pointer | F2 | System Common |

Song Position Pointer (F2) is a System Common message, not a System Real Time message. It stands in `sync` because it says where in a song the pace is.

`sync` does not cover MIDI Time Code, Song Select, the tempo of a Universal MIDI Packet, or a clock that is not MIDI (analogue, network, OSC).

### What a word says

The point of view of `sync` is that of the device described. `midi_in` keeps the point of view of the host (section 9.3 of 0.7).

- A word in `follows`: the device acts on that message when it receives it.
- A word in `sends`: the device emits that message of its own. Passing on a message it has received, and nothing more, does not count.

A word says that the device can do so under at least one of its settings. It says neither which setting nor the present state of the device.

### A program installed on a host system

For a program installed on a host system, the descriptor that the `midi.descriptor` of a target names says what the device does in MIDI while that program runs (section 23.37 of 0.7: the device as that firmware speaks MIDI). It does not matter whether the host or the program handles a message, and the settings of the host count.

Three cases, on an invented program "Pulse" that runs on an invented host:

| What happens | What the descriptor of its MIDI writes |
|---|---|
| The host derives a tempo from the clock it receives, and Pulse plays at that tempo | `timing_clock` in `follows` |
| While Pulse runs, the host emits the clock when its setting says so | `timing_clock` in `sends` |
| The host receives Stop, and nothing comes of it while Pulse runs | no `stop` in `follows` |

With these three facts alone, the member reads:

```json
{ "sync": { "follows": ["timing_clock"], "sends": ["timing_clock"] } }
```

### What absence and emptiness say

| Written | What it says |
|---|---|
| no `sync` | nothing |
| `"sync": {}` | nothing |
| a list absent from `sync` | nothing about what that list would say |
| `"follows": []` | the device follows none of the five messages |
| `"sends": []` | the device emits none of the five messages |

These messages have no channel. `sync` does not say through which port they pass.

No list requires a word of the other list, or another word of the same list: a device may follow `start` without `timing_clock`, or send what it does not follow.

### What a reader refuses

| Fault | At | Kind |
|---|---|---|
| a `sync` that is not an object | `/sync` | `BadValue` |
| a key of `sync` that is neither `follows`, `sends` nor a note | that key: `/sync/<key>` | `UnknownField` |
| a list that is not a list | `/sync/follows` or `/sync/sends` | `BadValue` |
| a word outside the five, an extension included, or a value that is not a string | the element: `/sync/follows/N` or `/sync/sends/N` | `BadValue` |
| a word written twice in one list | the second one: `/sync/follows/N` or `/sync/sends/N` | `BadValue` |
| a `sync` in a descriptor of 0.7 or earlier, whatever its value | `/sync` | `UnknownField` |

The schema refuses all six: the type of `sync` and of each list, the closed set of keys, the closed list of words, the repeated word (`uniqueItems`), and, in the schemas up to 0.7, the key itself at a root that admits no unknown key. The schema says that a text is refused. The kind and the address of each fault are those of the reader: `uniqueItems`, for one, judges the whole list, and the reader names the second occurrence.

A reader gives each list as written and writes it back as it was read.

### Cases

On an invented device, "Tock". Each line is the member of a descriptor of 0.8 that is otherwise valid, unless the line says another version.

| Written | Verdict |
|---|---|
| `"sync": {"follows": ["timing_clock", "start", "stop"], "sends": []}` | admitted: it follows three messages and sends none of the five |
| `"sync": {"sends": ["song_position_pointer"]}` | admitted: nothing is said of what it follows |
| `"sync": {"follows": ["stop", "timing_clock"]}` | admitted: the same claim as in the other order |
| `"sync": {}` | admitted: it says nothing |
| `"sync": {"follows": [], "_note": "checked with the clock of a sequencer"}` | admitted |
| `"sync": ["timing_clock"]` | refused: `BadValue` at `/sync` |
| `"sync": null` | refused: `BadValue` at `/sync` |
| `"sync": {"receives": []}` | refused: `UnknownField` at `/sync/receives` |
| `"sync": {"follows": "timing_clock"}` | refused: `BadValue` at `/sync/follows` |
| `"sync": {"sends": null}` | refused: `BadValue` at `/sync/sends` |
| `"sync": {"follows": ["timing_clock", "x-tock-beat"]}` | refused: `BadValue` at `/sync/follows/1` |
| `"sync": {"follows": [248]}` | refused: `BadValue` at `/sync/follows/0` |
| `"sync": {"sends": ["start", "stop", "start"]}` | refused: `BadValue` at `/sync/sends/2` |
| `"sync": {"follows": ["timing_clock"]}` in a descriptor of 0.7 | refused: `UnknownField` at `/sync` |

### Resolution

`sync` joins the mechanism of the blocks `identity` and `programs` (sections 23.8, 23.9 and 23.39 of 0.7):

- A file with `midi_of` may write `sync`. Its `sync` replaces the one of its target whole. The lists are not joined: a list that the file does not write is absent from the result, even when the target writes it.
- A file with `midi_of` that writes no `sync` leaves the result with the `sync` of its target, when the target has one.
- `sync` is thus one more key that a file with `midi_of` may hold beside its routes, with the two replaceable blocks (section 23.9).
- The MIDI file of a module of an assembly is such a file (sections 23.37 to 23.39). It may write `sync`, which replaces whole the one accumulated so far, that of the stock or of an earlier module. A community firmware that adds synchronisation says so in this way. A module that writes no `sync` leaves it as it is. Of two modules that write it, the later in the listed order wins, and neither is refused.
- No profile gives `sync`. A profile completes the blocks `identity` and `programs` only.

The version of each file is judged on its own text (sections 23.2 and 23.33): a file of 0.7 that writes `sync` is refused, even over a target of 0.8. A file of 0.7 without `sync` over a target of 0.8 keeps the `sync` of that target.

For an invented device whose target writes `"sync": {"follows": ["timing_clock"], "sends": ["timing_clock"]}`:

| The file with `midi_of` writes | The result holds |
|---|---|
| no `sync` | `{"follows": ["timing_clock"], "sends": ["timing_clock"]}` |
| `"sync": {"follows": ["start", "stop"]}` | `{"follows": ["start", "stop"]}`, with no `sends` |
| `"sync": {}` | `{}` |

For an assembly whose stock MIDI writes `"sync": {"sends": ["timing_clock"]}`:

| The MIDI files of its modules, in the listed order, write | The result holds |
|---|---|
| no `sync` | `{"sends": ["timing_clock"]}` |
| one writes `"sync": {"follows": ["timing_clock"]}` | `{"follows": ["timing_clock"]}`, with no `sends` |
| the first writes `"sync": {"follows": ["timing_clock"]}`, the second `"sync": {"follows": ["start"]}` | `{"follows": ["start"]}` |

### What `sync` leaves unchanged

| Field | What it says | Where |
|---|---|---|
| `sync` | what the device can follow and send, under at least one setting | a device descriptor |
| `sends.clock` | observed: the device emits the clock (F8) unasked, even when stopped; a habit that tells models apart | a fingerprint |
| `expects: clock` | the control takes a clock or tempo input (section 6.4) | a control |
| `midi_in` | the messages the device sends that are reported, from the point of view of the host | a device descriptor |

`sync` changes none of the other three. `sends.clock` of a fingerprint stays as it is. None is deduced from another, and a reader does not compare them: a fingerprint may say `"clock": false` of a device whose descriptor names `timing_clock` in `sends`, since one setting of the device is enough for the second.

`identity.sends`, which 0.7 removes (section 5.3.2), stays removed.

### The converters

The converter from 0.7 writes no `sync`, from any fingerprint or note. It loses nothing and adds nothing for this field.

The import of a MIDNAM document writes no `sync`. A converter to a format that does not carry `sync` names it among its losses, at `/sync`.

### The example: awake

awake is a program installed on a host system, norns. The example is the device descriptor of its MIDI, the one that the `midi.descriptor` of a target of its firmware descriptor would name (section 19). It does not write `device.kind: software`, which is the form of a program that no target ties to a device. The firmware descriptor carries no `sync`, and this draft does not decide its `kind`.

One fact was reported, to the project, on 2026-11-27: awake sends notes and follows the clock. The members that this draft touches or sets beside it:

```json
{
  "device": { "name": "awake", "osc_prefix": "/awake" },
  "midi_in": ["note_on", "note_off"],
  "sync": { "follows": ["timing_clock"] }
}
```

- The report does not say which clock is meant. `timing_clock` in `follows` is the reading this draft makes of it, to be confirmed.
- Whether awake, with its host, acts on any other of the five messages is not said. `sends` is absent: nothing is said of what is sent.
- The notes are said outside `sync`, in `midi_in`. The report speaks of notes; the two words written there are this draft's reading.
- The name and the OSC prefix are chosen for the example.

The script was not read again at its source. A public source with its revision is to be added, under `sources`, before the proposal is a candidate.

## What a reference does

| Reference | What it does | Read at its source for this draft |
|---|---|---|
| MIDI 1.0 | Defines the messages: Timing Clock, Start, Continue and Stop as System Real Time, Song Position Pointer as System Common, with the status bytes of the table above. | No. The names and bytes are written from knowledge of the specification, to be checked against it before the proposal is a candidate. |
| MIDNAM (`MIDIEvents10.dtd`) | The elements `TimingClock`, `Start`, `Continue`, `Stop` and `SongPositionPointer` are messages. None declares what a device follows or sends. The command lists of `MIDINameDocument10.dtd` admit none of the five: `MIDICommands` takes delays, channel messages and SysEx, `PatchMIDICommands` delays and channel messages. | Yes: the two DTD files kept in `osd-formats/tests/data/`. |
| MIDI implementation chart | A table of a device's manual with a column for what is transmitted and one for what is recognized, with rows for the system messages. | No. To be read at its source before the proposal is a candidate. |
| MIDI 2.0 Property Exchange | The properties of a controller (M2-117-UM), as section 22.1 of 0.7 lists them, hold none that says a device follows or sends the clock. Its other resources were not examined. | No. Taken from the table of section 22.1 of 0.7. The controller resources and the other resources are to be read before the proposal is a candidate. |

Where the draft departs:

- **The words `follows` and `sends`**, where the ways of a control say `transmit` and `recognize` (section 5.3.3) and the implementation chart says transmitted and recognized. This is the owner's decision. The reason: `transmit` and `recognize` say how a value is coded (`absolute`, `relative`, `both`, `none`), and these messages carry no such coding. `follows` asks more than recognizing, since the device acts on the message; `sends` leaves out a message that is only passed on.
- **From MIDNAM**, which declares the messages and no ability of a device. Reason: whoever patches two devices needs to know which one acts on the clock and which one emits it.
- **From the implementation chart**, which marks each message in a row. Reason: two lists of words are structure that the schema checks, and an absent list says that nothing is known, apart from an empty one that says none.
- **From MIDI 2.0 Property Exchange**, from whose controller properties nothing is taken. Reason: the clock is not a controller, and a description is read with no device in hand.

## Open points

- **A word that a list does not hold.** This draft says what a word claims and what an empty list says. It does not yet say whether a list that holds some words denies the others or leaves them unsaid. Denying them makes each list complete, as an empty list is, and the example of awake would then say that it does not act on Start, which no report says. Leaving them unsaid lets a writer say only what is known; this is the reading proposed. It is to be decided before the proposal is a candidate.
- **The example of awake.** Which clock it follows is to be confirmed, and its public source with its revision added.
- **The references** marked as not read are to be read at their source.

## Objections

None yet.
