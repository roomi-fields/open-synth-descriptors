# 0021 — A source or a destination of modulation carries its meaning

## Status

- State: `draft`
- Candidate since: —
- Under the trial: —

## What it changes

The text and the schemas of 0.8, the version that follows 0.7, and state 3 of the vocabulary of roles, the state after [state 2](../vocabulary/2.md): a named value of a source selector or of a destination selector of modulation gains one field, `source` or `destination`, written in terms of the vocabulary.

## Why

What this brings to the sharing of sounds on devices: "destination 12" reads "filter cutoff" on every device, so an agent can read a modulation matrix and say what each route does.

In 0.7 the choices of a selector are named values, `{ "value": integer, "title": string }` (section 5.3.3 of [0.7](../0.7.md)). The title is the maker's word, free text: nothing says that the choice "Mod Env 1" of one device and "env 3" of another are envelopes, or that "FiltFreq" is the cutoff of the filter. A control has no role that says it selects the source or the destination of a route.

## The change

### The routes stay ordinary settings

This proposal adds no block of routes. A route is made of ordinary controls, each with its ways, told apart by their roles:

| Control | Role |
|---|---|
| the source selector of route n | parts `route` n, setting `source` |
| the destination selector of route n | parts `route` n, setting `destination` |
| the quantity of route n | parts `route` n, setting `amount` |

A control is a **source selector** when the last part of its role is `route` and the setting is `source`, and a **destination selector** when the setting is `destination`. A publisher MUST give each route its number, from 1, in the `index` of the part `route`, a device with one route included. The controls of one route are those whose roles have equal `parts`, numbers included; a reader brings them together by that. A route of one track is `/Track/2/Route/1/Source`, under the rule of [section 7 of state 2](../vocabulary/2.md) on tracks and contexts.

### The two fields of a named value

A named value of a control gains two optional fields. Each applies to one kind of selector.

| Field | On | Is |
|---|---|---|
| `source` | a value of a source selector | an object `{ "parts": [...], "setting"?: term }`: the parts as in a role, each with its `index` when the device has several, then a setting only when the source is a gesture of play |
| `destination` | a value of a destination selector | a role, `{ "parts": [...], "setting": term }`: what the route moves |

Both are objects of the vocabulary, read against the state that `role_vocabulary` names, never strings. The `title` stays the maker's word, and the `value` stays the number of the message. A value with no shared meaning (no source, nothing routed, a constant, a function this proposal does not name, a choice whose meaning is unknown) carries neither field. Two values may carry the same object (a unipolar and a bipolar choice of one LFO): what tells them apart stays in the title.

A publisher MUST write a `source` with no `setting` only for a part whose signal is the source (an LFO, an envelope), and a `source` with a `setting` only for a gesture of play. A publisher MUST write what its source document says, and MUST NOT read a meaning from a title alone. The number of a part is the maker's (LFO 2 is `lfo` with `index` 2).

### The schema

The fragment shows what is added to the device schema. The two properties join those of the items of `named_values`, whose `required` and `additionalProperties` do not change, and `source_path` joins the `$defs`. `role` and `role_part` are the definitions of 0.7, unchanged.

```json
{
  "$defs": {
    "named_values": {
      "items": {
        "properties": {
          "source": { "$ref": "#/$defs/source_path" },
          "destination": { "$ref": "#/$defs/role" }
        }
      }
    },
    "source_path": {
      "type": "object",
      "required": ["parts"],
      "additionalProperties": false,
      "patternProperties": { "^_": true },
      "properties": {
        "parts": { "type": "array", "minItems": 1, "items": { "$ref": "#/$defs/role_part" } },
        "setting": { "type": "string", "pattern": "^[a-z][a-z0-9-]*$" }
      }
    }
  }
}
```

### An example

An invented device, "Orbe". The first route, its source and its destination:

```json
{
  "role_vocabulary": "3",
  "cc_params": {
    "entries": [
      { "osc": "/mod/1/source",
        "role": { "parts": [{ "part": "route", "index": 1 }], "setting": "source" },
        "ways": [{ "ctrlType": "cc", "ctrlIndex": [20] }],
        "values": [
          { "value": 0, "title": "Off" },
          { "value": 1, "title": "LFO 2", "source": { "parts": [{ "part": "lfo", "index": 2 }] } },
          { "value": 2, "title": "Flt Env", "source": { "parts": [{ "part": "filter" }, { "part": "envelope" }] } },
          { "value": 3, "title": "Wheel",
            "source": { "parts": [{ "part": "performance" }], "setting": "modulation" } },
          { "value": 4, "title": "Breath",
            "source": { "parts": [{ "part": "performance" }], "setting": "breath" } },
          { "value": 5, "title": "Orbit" }
        ] },
      { "osc": "/mod/1/destination",
        "role": { "parts": [{ "part": "route", "index": 1 }], "setting": "destination" },
        "ways": [{ "ctrlType": "cc", "ctrlIndex": [21] }],
        "values": [
          { "value": 0, "title": "---" },
          { "value": 1, "title": "Cutoff",
            "destination": { "parts": [{ "part": "filter" }], "setting": "cutoff" } },
          { "value": 2, "title": "O2 PW",
            "destination": { "parts": [{ "part": "oscillator", "index": 2 }], "setting": "pulse-width" } },
          { "value": 3, "title": "LFO1 Dpt",
            "destination": { "parts": [{ "part": "lfo", "index": 1 }], "setting": "amount" } }
        ] }
    ]
  }
}
```

"Off", "Orbit" and "---" carry no meaning: "Orbit" is a source of Orbe alone.

### A free matrix of two routes

Each route has three controls, each with its whole role. A reader brings together the controls whose part `route` has the same number. The values of the selectors are written as above and left out here.

```json
[
  { "osc": "/mod/1/source", "role": { "parts": [{ "part": "route", "index": 1 }], "setting": "source" },
    "ways": [{ "ctrlType": "cc", "ctrlIndex": [20] }] },
  { "osc": "/mod/1/destination", "role": { "parts": [{ "part": "route", "index": 1 }], "setting": "destination" },
    "ways": [{ "ctrlType": "cc", "ctrlIndex": [21] }] },
  { "osc": "/mod/1/amount", "role": { "parts": [{ "part": "route", "index": 1 }], "setting": "amount" },
    "polarity": "bipolar", "ways": [{ "ctrlType": "cc", "ctrlIndex": [22] }] },
  { "osc": "/mod/2/source", "role": { "parts": [{ "part": "route", "index": 2 }], "setting": "source" },
    "ways": [{ "ctrlType": "cc", "ctrlIndex": [23] }] },
  { "osc": "/mod/2/destination", "role": { "parts": [{ "part": "route", "index": 2 }], "setting": "destination" },
    "ways": [{ "ctrlType": "cc", "ctrlIndex": [24] }] },
  { "osc": "/mod/2/amount", "role": { "parts": [{ "part": "route", "index": 2 }], "setting": "amount" },
    "polarity": "bipolar", "ways": [{ "ctrlType": "cc", "ctrlIndex": [25] }] }
]
```

### Routes with one end fixed

Some devices let one end of a route be chosen and fix the other. In both forms below, the chosen end is a selector and the quantity is the `amount` of the same route. **The fixed end, which has no named value, is not said by this proposal**: no field is added to a control.

**One source to several destinations** (a wheel, a pressure or an LFO that offers a few destination slots, each with its quantity). Slot n is route n:

| Control | Role |
|---|---|
| the destination of slot 1 | `/Route/1/Destination`, with `destination` on its values |
| the quantity of slot 1 | `/Route/1/Amount` |
| the destination of slot 2 | `/Route/2/Destination` |
| the quantity of slot 2 | `/Route/2/Amount` |

What an existing role already says of the fixed source: where the source is a gesture of play, the control that is that gesture has its role (`/Performance/Modulation` for the wheel), which nothing ties to the routes; where the source is an LFO, `/Lfo/Amount` on a quantity names it. But a control has one role, and a quantity written `/Lfo/Amount` has no route number to join it to its destination: a publisher that writes the selector as a route writes the quantity as `/Route/n/Amount`. The rest, that the routes start at this source, is left to a later proposal.

**One source per destination** (each of several destinations has its own selector of source and its quantity). Each such pair is one route:

| Control | Role |
|---|---|
| the source chosen for the first destination | `/Route/1/Source`, with `source` on its values |
| its quantity | `/Route/1/Amount` |
| the source chosen for the second destination | `/Route/2/Source` |
| its quantity | `/Route/2/Amount` |

What a role already says: the destination parameter has its own role (`/Filter/Cutoff`) on its own control, which nothing ties to the route; and where both ends are fixed, an existing role says both (`/Filter/Envelope/Amount`, `/Filter/Lfo/Amount`), with no selector. The rest, which destination a route with a chosen source moves, is left to a later proposal.

### What state 3 adds to the vocabulary

State 3 keeps every term of state 2 and adds the following. A function that state 2 already names, by a term or a composition, is taken as it is, with no new term: `lfo`, `envelope`, `vibrato`, `performance` with `modulation`, `breath` or `expression`, and the settings of the filter, the oscillator, the amplifier, the output and the effects. A new term that stands for a source of DLS Level 2 takes the word of its identifier, without the prefix, in the form of the vocabulary (the last part of `CONN_SRC_PITCHWHEEL` is `pitch-wheel`).

The documents named in the last column: DLS is the MIDI Manufacturers Association's Downloadable Sounds Level 2, version 2.2 (April 2006); the others are listed under "The sources of the needs" below.

| Addition | Type | Definition | `applies_to` or `stands_under` | Source (document and section, or `shared_by`) | Why it is missing |
|---|---|---|---|---|---|
| `route` | part | A connection that takes one source of modulation to one destination, numbered from 1. | opens a path; stands under `track` | DLS, section 1.6.2, connection block | no part of state 2 stands for a connection |
| `source` | setting | Which signal or gesture a route takes. | `route` | DLS, section 1.6.2, `usSource` | no setting selects an input |
| `destination` | setting | Which setting a route moves. | `route` | DLS, section 1.6.2, `usDestination` | no setting selects a target |
| `amount` (existing) | setting | How strongly a part moves the setting it is routed to. | gains `route` | DLS, section 1.6.2, `lScale` | the quantity of a route is this function; a second term would let it in twice |
| `envelope` (existing) | part | An envelope that shapes a setting of the part it stands under over the life of a note. | now also opens a path; keeps what it stands under | Circuit Tracks guide, "Mod Matrix Table", `env 3` | an envelope tied to no part (`/Envelope/3`) cannot be written |
| `level` (existing) | setting | How loud a part is, the output as a whole or one source or effect in it. | gains `amplifier` | DLS, section 4.2, `CONN_DST_GAIN` | the gain of the amplifier stage cannot be written |
| `note` | part | A note as it is played, with its key and the way it was struck. | opens a path | DLS, section 4.1, `CONN_SRC_KEYONVELOCITY` and `CONN_SRC_KEYNUMBER` | velocity and key come with a note, and `performance` is what a player moves apart from the notes |
| `key-on-velocity` | setting | How hard a key is struck when a note starts. | `note` | DLS, section 4.1, `CONN_SRC_KEYONVELOCITY` | a gesture of play with no term |
| `key-number` | setting | Which key a note is played on. | `note` | DLS, section 4.1, `CONN_SRC_KEYNUMBER` | a gesture of play with no term |
| `pitch-wheel` | setting | Where the player holds the pitch wheel. | `performance` | DLS, section 4.1, `CONN_SRC_PITCHWHEEL` | a gesture of play with no term |
| `channel-pressure` | setting | How hard the player presses the keys after striking them, as one value for the channel. | `performance` | DLS, section 4.2, `CONN_SRC_CHANNELPRESSURE` | a gesture of play with no term |
| `poly-pressure` | setting | How hard the player presses one key after striking it, for that note alone. | `performance` | DLS, section 4.2, `CONN_SRC_POLYPRESSURE` | a gesture of play with no term |
| `pulse-width` | setting | How much of each period a pulse wave stays high. | `oscillator` | Circuit Tracks guide, "Mod Matrix Table", `osc 1 pulse width / index`; `shared_by` Novation Circuit Tracks and Synthstrom Deluge | DLS names no destination for it and state 2 no setting |

### The sources of the needs

The documents are cited, never copied. They were read on 2026-11-27.

- **Circuit Tracks guide**: Novation, *Circuit Tracks Programmer's Reference Guide*, version 3, "Mod Matrix Table".
- **Peak guide**: Novation, *Peak User Guide* for firmware 1.2, appendix, "Modulation Matrix – sources", "Modulation Matrix – destinations", "FX Modulation Matrix – destinations".
- **Deluge manual**: Synthstrom Audible, *Deluge Official Manual*, 4.0, section 6, "Modulation Source Shortcuts" and the table "Modulation Sources".
- **MiniFreak manual**: Arturia, *MiniFreak User Manual*, 4.0.1, sections 8.5.4 and 8.5.5.

### DLS Level 2 against the needs, function by function

"§" is a section of DLS Level 2, whose header files list the identifiers: §4.1 those of Level 1, §4.2 those Level 2 adds. A role is written here as its `paramPath`; `n` is the maker's number.

#### Sources

| Need read | Public source | DLS 2 identifier and section | Written as (`source`) | Missing, and why |
|---|---|---|---|---|
| a low-frequency oscillator, one of several | Circuit Tracks guide: `LFO 1`, `LFO 2`; Peak guide: `Lfo1` to `Lfo4` | `CONN_SRC_LFO`, §4.1: covered for one LFO | `/Lfo/n` | the numbers beyond one: the `index` says them |
| the amplitude envelope | Circuit Tracks guide: `env amp`; Peak guide: `AmpEnv` | `CONN_SRC_EG1`, §4.1: covered | `/Amplifier/Envelope` | — |
| the filter envelope | Circuit Tracks guide: `env filter` | `CONN_SRC_EG2`, §4.1: covered (DLS uses it for pitch and filter) | `/Filter/Envelope` | — |
| a further envelope | Circuit Tracks guide: `env 3`; Peak guide: `ModEnv1`, `ModEnv2` | not covered: two envelope generators | `/Envelope/n` | `envelope` opens a path |
| velocity | Circuit Tracks guide: `velocity`; Peak guide: `Velocity` | `CONN_SRC_KEYONVELOCITY`, §4.1: covered | `/Note/KeyOnVelocity` | — |
| keyboard tracking | Circuit Tracks guide: `keyboard`; Deluge manual: `NOTE` | `CONN_SRC_KEYNUMBER`, §4.1: covered | `/Note/KeyNumber` | — |
| modulation wheel | Peak guide: `ModWheel` | `CONN_SRC_CC1`, §4.1: covered | `/Performance/Modulation` | — |
| pitch bend | Peak guide: `BndWhl+`, `BndWhl-` | `CONN_SRC_PITCHWHEEL`, §4.1: covered | `/Performance/PitchWheel` | the direction stays in the title |
| aftertouch: of the channel, of one key, or whichever arrives | Peak guide: `AftTouch`, and "The Modulation Matrix"; Deluge manual: `AFTERTOUCH` | `CONN_SRC_CHANNELPRESSURE` and `CONN_SRC_POLYPRESSURE`, §4.2: covered one by one | `/Performance/ChannelPressure` or `/Performance/PolyPressure`, for a choice its document gives to one of the two | the two cited documents describe a choice that takes either: such a choice carries no `source` (open points) |
| a control the player assigns | Peak guide: `Animate1`, `Animate2` | not covered | nothing | what it moves is the maker's or the player's choice: no shared meaning |

#### Destinations

| Need read | Public source | DLS 2 identifier and section | Written as (`destination`) | Missing, and why |
|---|---|---|---|---|
| the pitch of the whole voice | Circuit Tracks guide: `osc 1 & 2 pitch`; Peak guide: `O123Ptch` | `CONN_DST_PITCH`, §4.1: covered | `/Voice/Pitch` | — |
| the pitch of one oscillator | Circuit Tracks guide: `osc 1 pitch`; Peak guide: `Osc1Ptch` | not covered: one pitch | `/Oscillator/n/Pitch` | existing terms |
| pulse width | Circuit Tracks guide: `osc 1 pulse width / index`; Deluge manual: `Pulse Width` | not covered | `/Oscillator/n/PulseWidth`, `/Oscillator/PulseWidth` | new setting `pulse-width` |
| the cutoff of the filter | Circuit Tracks guide: `filter frequency`; Peak guide: `FiltFreq` | `CONN_DST_FILTER_CUTOFF`, §4.2: covered | `/Filter/Cutoff` | — |
| the resonance of the filter | Circuit Tracks guide: `filter resonance`; Peak guide: `Filt Res` | `CONN_DST_FILTER_Q`, §4.2: covered | `/Filter/Resonance` | — |
| the level of the amplifier | MiniFreak manual, 8.5.4: `VCA`; Peak guide: `VcaLevel` | `CONN_DST_GAIN`, §4.2, the same number as `CONN_DST_ATTENUATION`, §4.1: covered | `/Amplifier/Level` | `level` applies to `amplifier` |
| the level of one source | Circuit Tracks guide: `osc 1 level`, `noise level`; Peak guide: `NoiseLev` | not covered | `/Oscillator/n/Level`, `/Noise/Level` | existing terms |
| pan | Deluge manual: `Pan` | `CONN_DST_PAN`, §4.1: covered | `/Output/Pan` | — |
| the rate of an LFO | Circuit Tracks guide: `LFO 1 rate`; Peak guide: `Lfo1Rate` | `CONN_DST_LFO_FREQUENCY`, §4.1: covered for one LFO | `/Lfo/n/Rate` | — |
| the depth of an LFO | MiniFreak manual, 8.5.4: `LFO1 AM` | not covered as a destination: DLS scales a connection by its control input (§1.6.2) | `/Lfo/n/Amount` | existing terms |
| the attack of an envelope | Peak guide: `AmpEnv A`, `ModEnv1A` | `CONN_DST_EG1_ATTACKTIME`, `CONN_DST_EG2_ATTACKTIME`, §4.1: covered for two | `/Amplifier/Envelope/AttackTime`, `/Filter/Envelope/AttackTime`, `/Envelope/n/AttackTime` | further envelopes: `envelope` opens a path |
| the decay of an envelope | Circuit Tracks guide: `amp envelope decay`, `filter envelope decay`; Peak guide: `AmpEnv D` | `CONN_DST_EG1_DECAYTIME`, `CONN_DST_EG2_DECAYTIME`, §4.1: covered for two | the same paths, `DecayTime` | as above |
| the sustain of an envelope | Deluge manual: `ENV 1/2 ADSR`; MiniFreak manual, 8.5.5: `Envelope Sustain` | `CONN_DST_EG1_SUSTAINLEVEL`, `CONN_DST_EG2_SUSTAINLEVEL`, §4.1: covered for two | the same paths, `SustainLevel` | as above |
| the release of an envelope | Peak guide: `AmpEnv R`, `ModEnv1R` | `CONN_DST_EG1_RELEASETIME`, `CONN_DST_EG2_RELEASETIME`, §4.1: covered for two | the same paths, `ReleaseTime` | as above |
| how much goes to the reverb | Deluge manual: `Reverb Amount` | `CONN_DST_REVERB`, §4.2: covered | `/Reverb/Depth` for a send; `/Reverb/Level` where the document gives the level of the effect (Peak guide: `Rev Lev`) | existing terms |
| how much goes to the chorus | Peak guide, FX destinations: `Chor Lev`, `Chor Dep` | `CONN_DST_CHORUS`, §4.2: covers the send | `/Chorus/Depth` for a send; `/Chorus/Level` for the level of the effect (Peak guide: `Chor Lev`); `/Chorus/ModulationDepth` for the depth of its movement (Peak guide: `Chor Dep`) | existing terms, `modulation-depth` among them (state 2) |
| a setting of an effect | Peak guide, FX destinations: `Del Time`, `Del FB`, `Rev Time`, `ChorRate` | not covered | `/Delay/DelayTime`, `/Delay/Feedback` | no term of state 2 names the time of a reverb or the rate of a chorus: such a choice carries no `destination`, or one with an extension |

#### What DLS Level 2 names and no need read calls for

State 3 adds no term for these. Where state 2 already names the function, a publisher whose source states it writes it.

| DLS 2 identifier and section | Written, where a document states it |
|---|---|
| `CONN_SRC_NONE`, `CONN_DST_NONE`, §4.1 | nothing: the value carries no field |
| `CONN_SRC_CC11`, §4.1 | `/Performance/Expression` |
| `CONN_SRC_VIBRATO`, §4.2 | `/Vibrato` |
| `CONN_DST_VIB_FREQUENCY`, `CONN_DST_VIB_STARTDELAY`, §4.2 | `/Vibrato/Rate`, `/Vibrato/DelayTime` |
| `CONN_SRC_CC7`, `CONN_SRC_CC10`, `CONN_SRC_RPN0`, `CONN_SRC_RPN1`, `CONN_SRC_RPN2`, §4.1; `CONN_SRC_CC91`, `CONN_SRC_CC93`, `CONN_SRC_MONOPRESSURE`, §4.2 | nothing yet |
| `CONN_DST_LFO_STARTDELAY`, §4.1; `CONN_DST_EG1_DELAYTIME`, `CONN_DST_EG1_HOLDTIME`, `CONN_DST_EG1_SHUTDOWNTIME`, `CONN_DST_EG2_DELAYTIME`, `CONN_DST_EG2_HOLDTIME`, §4.2 | nothing yet |
| `CONN_DST_KEYNUMBER`, `CONN_DST_LEFT`, `CONN_DST_RIGHT`, `CONN_DST_CENTER`, `CONN_DST_LEFTREAR`, `CONN_DST_RIGHTREAR`, `CONN_DST_LFE_CHANNEL`, §4.2 | nothing yet |
| `CONN_DST_RESERVED`, `CONN_DST_EG1_RESERVED`, `CONN_DST_EG2_RESERVED`, §4.1 | reserved by the standard |

### What a reader refuses

`E` stands for `/cc_params/entries/N/values/K` or `/params/entries/N/values/K`, and `F` for `E/source` or `E/destination`.

| Fault | At | Kind | Seen by |
|---|---|---|---|
| `source` or `destination` on a control with no role | the `role` of the control (`/cc_params/entries/N/role`, `/params/entries/N/role`) | `Missing` | the reader |
| `source` on a control that is not a source selector | `E/source` | `BadValue` | the reader |
| `destination` on a control that is not a destination selector | `E/destination` | `BadValue` | the reader |
| a `source` or `destination` that is no object (a string included) | `F` | `BadValue` | the schema and the reader |
| no `parts`; a `destination` with no `setting` | `F/parts`, `E/destination/setting` | `Missing` | the schema and the reader |
| a part with no `part` | `F/parts/J/part` | `Missing` | the schema and the reader |
| empty `parts` | `F/parts` | `BadValue` | the schema and the reader |
| an `index` that is not an integer of 1 or more | `F/parts/J/index` | `BadValue` | the schema and the reader |
| a key the object does not define | the key | `UnknownField` | the schema and the reader |
| a `part` or a `setting` that has not the form of a term or of an extension | `F/parts/J/part`, `F/setting` | `BadValue` | the schema and the reader |
| a term the state does not hold, or of another type | `F/parts/J/part`, `F/setting` | `BadValue` | the reader |
| a part that does not stand where its term says | `F/parts/J` | `BadValue` | the reader |
| a setting that does not apply to the last part | `F` | `BadValue` | the reader |
| `source` or `destination` in a descriptor of 0.7 or earlier | `E/source`, `E/destination` | `UnknownField` | the schema of that version and the reader |

The schema sees the form of the two objects. It does not see the role of the control against the field, nor the terms against the state: those are the reader's alone.

**Extensions.** A part or a setting of a `source` or a `destination` may be an extension `x-<registrant>-<term>`, in the form and the registry of the roles (`schemas/extensions.md`). A reader accepts it, gives the object as written, and reads it as no known source or destination; it checks that every other term is a term of the state, and checks nothing of what the extension touches. On a control whose role holds an extension, a reader does not judge whether the control is a selector: it keeps the fields and checks their form and their terms.

A reader gives `source` and `destination` as written and writes them back.

### The converters

A converter from 0.7 writes no `source` and no `destination`, writes no role of a route, and guesses none from a title, an address or a name of a value. A converter to a format that has no such field (0.7 and earlier, MIDNAM, OSCQuery, Property Exchange) writes the value and its title unchanged and reports each `source` and each `destination` as lost, at its pointer.

The field `term` of [draft 0020, a named value carries a term](0020-a-named-value-carries-a-term-of-the-vocabulary.md) is not used here: a source or a destination is a path, which one term cannot say.

### Open points

- A choice that takes channel or polyphonic pressure, whichever arrives, has no term: it would repeat the two. To be decided before the proposal is a candidate.
- Some devices give a route a second source, which multiplies or scales the first (Peak guide, "The Modulation Matrix"; the `usControl` of a DLS connection block). This proposal gives it no setting: its control carries no role of a route.
- Whether state 3 is the state these terms enter, or state 2 while it is not published, is the owner's choice.

## What a reference does

| Reference | What it does of the need | Read for this draft |
|---|---|---|
| DLS Level 2, version 2.2 | A connection block names a source, a control, a destination, a transform and a scale (§1.6.2). Sources and destinations are closed lists of numbers with names (§4.1, §4.2): LFOs, two envelope generators, velocity, key number, the wheel, pressures, a few controllers; gain, pitch, pan, filter, LFO and envelope settings, sends. | Read at the source on 2026-11-27. |
| GM2, Controller Destination Setting (C/A-022), as General MIDI 2 v1.2a states it, §4.6 | A message routes channel pressure, polyphonic pressure or a control change to pitch, filter cutoff, amplitude, and the depth of the LFO on pitch, filter and amplitude, each with a range. | §4.6 of General MIDI 2 read on 2026-11-27. The text of C/A-022 itself was not read, to be read before the proposal is a candidate. |
| SoundFont 2.04 (sfspec24.pdf, from synthfont.com: sections 7.4, 8.1, 8.2) | A modulator has a source, a destination, an amount, a second source of amount and a transform. Its sources are controllers (velocity, key number, pressures, pitch wheel and its sensitivity, MIDI control changes, a link to another modulator, or none) and its destinations are generators or another modulator; the LFOs and envelopes act through generators of their own (`modLfoToPitch`), not as modulator sources. | Read at the source on 2026-11-27. |
| MIDI 2.0 Property Exchange, controller resources (M2-117-UM v1.0) | A controller has a `paramPath`; a choice of `CtrlMapList` is a value and a title, with no field for its meaning. | A text extract of the document read on 2026-11-27. |
| MIDNAM (`MIDINameDocument10.dtd`) | A `Value` of a `ValueNameList` has a `Number` and a `Name`. | The DTD kept in `osd-formats/tests/data/`, read on 2026-11-27. |
| OSCQuery (github.com/Vidvox/OSCQueryProposal, `README.md`) | `RANGE` may list the acceptable values (`VALS`); `EXTENDED_TYPE` describes what a value of the method means, one string per argument, not per choice. | Read at the source on 2026-11-27. |

**What OSD takes**: the two ends of a DLS connection block and its scale, as the settings `source`, `destination` and `amount` of a route; the sources and destinations DLS names, written in the words of the vocabulary; a numbered vocabulary that a reader checks, as DLS has closed lists.

**Why DLS Level 2 rather than C/A-022.** C/A-022 routes a controller of MIDI to six destinations. It names no source inside the instrument (no LFO, envelope, velocity or key number) and few destinations. DLS names both ends of a connection inside the instrument, which is what a matrix does. The depths of an LFO that C/A-022 lists as destinations are written here with `amount`.

**Where the draft departs, and why.**

- **No connection block.** DLS, SoundFont and C/A-022 describe the routes as data. Here a route is made of the device's own settings: a descriptor says how a device speaks, and the routes of a sound are the values of those settings.
- **A path rather than a number.** DLS and SoundFont number a fixed list (two envelopes, one filter). A device has any number of parts: the vocabulary composes them with their numbers, with the rule that a function is named once. So the two DLS envelopes are `/Amplifier/Envelope` and `/Filter/Envelope`, and no term repeats them.
- **No second source or transform of a connection.** The quantity is a setting of the device with its own ways. The second source and the curves are left to later.
- **From Property Exchange, MIDNAM and OSCQuery**, which give a choice a title alone, or a word for a whole argument. Reason: the shared meaning of each choice is what this proposal adds, beside the maker's title, in a structure a reader checks.

## Objections

None yet.
