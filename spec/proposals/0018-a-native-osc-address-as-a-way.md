# 0018 — A native OSC address as a way of a control

## Status

- State: `draft`
- Candidate since: —
- Under the trial: —

## What it changes

The text and the schemas of 0.8, the version that follows 0.7: the ways of section 5.3.3 of 0.7 (a ninth kind of message, `osc`, with `path` and `args`), and the rule that a message is claimed once (an OSC address is claimed once on each side).

## Why

What this brings to the sharing of sounds on devices: a software that speaks OSC with no MIDI behind it is described, so whoever receives its descriptor can receive its messages and send them.

0.7 already links an address on the side of a tool to a message, in two places. A command with `forward` takes an address on the side of the tool and sends an OSC message to a software. A reply with `match_osc` recognises an OSC message from the software and reports it at an address on the side of the tool. Each works in one direction, at its own place, and no rule says that an address is claimed once.

A way of kind `osc` says in one place the native message of a control: its address, its arguments, both its directions and the contexts it holds in. `commands` (`forward`) and `replies` (`match_osc`) stay as they are.

## The change

Everything of 0.7 that this section does not name stays as it is.

### The kind

`ctrlType` of a way gains a ninth word, `osc`. A way of that kind is a way of an entry of `cc_params`, beside the ways that are messages of MIDI, in the same list and in the same order of preference. The sentences of section 5.3.3 that say that a `cc_params` entry is reached by the kinds of MIDI, and that `ctrlType` is one of eight words, change accordingly. A `params` entry does not take it.

| Field of an `osc` way | Says |
|---|---|
| `path` | required: the OSC address of the message |
| `args` | required: the arguments of the message, a list of objects, in the order of the message; `[]` is a message with no argument |
| `range` | optional: lowest and highest value of the argument that carries the value of the control |
| `transmit`, `recognize` | the direction: `absolute` or `none` |
| `in`, `slice` | the contexts the way holds in, as on any way of `cc_params` |

An `osc` way holds no `ctrlIndex`, no `value_bits` and no `nrpn_lsb_range`. It ignores the channel of its control and of its table: OSC has no channel.

```json
{
  "osc": "/filter/cutoff",
  "ways": [
    { "ctrlType": "cc", "ctrlIndex": [74] },
    { "ctrlType": "osc", "path": "/synth/cut", "args": [{ "type": "int", "control_value": true }], "range": [0, 1000] }
  ]
}
```

### The address

`path` is the whole address as it travels on the wire. A reader and a sender never prefix it with `device.osc_prefix`. It is a string that starts with `/` and holds no space, no brace and no pattern character of OSC 1.0 (`?`, `*`, `[`, `]`, `{`, `}`). It is compared as written, character by character. It is a string only: it takes no segment and no capture (section 9.1 of 0.7).

The field `osc` of the control stays required and keeps its meaning. The two differ:

| Field | Whose address | Prefix |
|---|---|---|
| `osc` of the control | the address of the control on the side of the tool, read alike from one device to the next | after `device.osc_prefix` |
| `path` of an `osc` way | the address the device or the software itself speaks | none: written whole |

### The arguments

Each element of `args` is an object. It has a `type`, from a closed list:

| `type` | OSC type tag | The argument |
|---|---|---|
| `int` | `i` | a 32-bit integer |
| `string` | `s` | a string |

The other types of OSC are open points. Beside `type`, the object holds **exactly one** of three keys:

| Key | Form | The argument |
|---|---|---|
| `const` | a value of the type: an integer from -2147483648 to 2147483647 for `int`, a string for `string` | a fixed value |
| `control_value` | `true` | carries the value of the control; `type` is `int`; at most one in a way, and none is admitted |
| `name` | `[A-Za-z_][A-Za-z0-9_]*`, the form of an argument name (section 9.1 of 0.7), unique in the list | a free argument, of which the descriptor says nothing else |

The last argument of the list, when it is free, MAY carry `count`, `[low, high]`: two integers with 0 ≤ low ≤ high and high ≥ 1. It is then repeated from `low` to `high` times. Without `count` it appears once.

An argument object accepts notes (keys that start with `_`) and no other key.

### The schema

In `$defs/cc_way` of the device schema of 0.8, `ctrlType` gains `"osc"` in its `enum`, and `properties` gains:

```json
{
  "path": {
    "type": "string",
    "pattern": "^/[^ ?*\\[\\]{}]*$",
    "description": "For an osc way: the whole OSC address of the message, as it travels, never prefixed with device.osc_prefix."
  },
  "args": {
    "type": "array",
    "items": { "$ref": "#/$defs/osc_argument" },
    "contains": { "type": "object", "required": ["control_value"] },
    "minContains": 0,
    "maxContains": 1,
    "description": "For an osc way: the arguments of the message, in order. Names are unique and only a last free argument takes count (the reader refuses the rest)."
  }
}
```

`$defs` gains:

```json
{
  "osc_argument": {
    "type": "object",
    "properties": {
      "type": { "enum": ["int", "string"] },
      "const": {},
      "control_value": { "const": true },
      "name": { "type": "string", "pattern": "^[A-Za-z_][A-Za-z0-9_]*$" },
      "count": {
        "type": "array",
        "prefixItems": [
          { "type": "integer", "minimum": 0 },
          { "type": "integer", "minimum": 1 }
        ],
        "minItems": 2,
        "maxItems": 2
      }
    },
    "patternProperties": { "^_": true },
    "additionalProperties": false,
    "required": ["type"],
    "oneOf": [
      { "required": ["const"] },
      { "required": ["control_value"] },
      { "required": ["name"] }
    ],
    "dependentRequired": { "count": ["name"] },
    "allOf": [
      {
        "if": { "required": ["type"], "properties": { "type": { "const": "int" } } },
        "then": { "properties": { "const": { "type": "integer", "minimum": -2147483648, "maximum": 2147483647 } } }
      },
      {
        "if": { "required": ["type"], "properties": { "type": { "const": "string" } } },
        "then": { "properties": { "const": { "type": "string" }, "control_value": false } }
      }
    ]
  }
}
```

In `allOf` of `cc_way`, two conditions of 0.7 gain `"osc"`: the one that forbids `ctrlIndex` (and requires it of every other kind), whose `enum` becomes `["chPress", "pBend", "osc"]`, and the one that forbids `value_bits`, whose `enum` becomes `["chPress", "pPress", "note", "osc"]`. The condition on `nrpn_lsb_range` stays and already forbids it on an `osc` way. These three conditions join the list:

```json
[
  {
    "if": { "required": ["ctrlType"], "properties": { "ctrlType": { "const": "osc" } } },
    "then": {
      "required": ["path", "args"],
      "properties": {
        "transmit": { "enum": ["absolute", "none"] },
        "recognize": { "enum": ["absolute", "none"] }
      }
    }
  },
  {
    "if": { "required": ["ctrlType"], "properties": { "ctrlType": { "not": { "const": "osc" } } } },
    "then": { "properties": { "path": false, "args": false } }
  },
  {
    "if": {
      "required": ["ctrlType", "args"],
      "properties": {
        "ctrlType": { "const": "osc" },
        "args": { "not": { "contains": { "type": "object", "required": ["control_value"] } } }
      }
    },
    "then": { "properties": { "range": false, "slice": false } }
  }
]
```

`sysex_way`, the way of a `params` entry, does not change.

### The value and the scale

The value of an `osc` way is that of its argument with `control_value`. `range`, optional, bounds that argument. `range` or `slice` on a way with no such argument is refused.

**With a `range`**, the way follows section 5.3.3 as any way does: it counts in the choice of the reference scale (the most precise way, when the control gives no `scale`), and a tool converts between the reference scale and the `range` by the affine scaling of that section.

**Without a `range`**, the way implies no extent. It does not count in the choice of the reference scale. Its integer is that of the reference scale, with no conversion and no clamping, in both directions.

A control carried only by `osc` ways without `range`, and with no `scale`, has **no known scale**: `default`, `values` and the `range` of the readings are compared to nothing, and the value passes as it is. With `scale`, they are compared to `scale`, and the integer on the wire still passes as it is.

On an invented device, "Pip". Each line is one control.

| Control | Reference scale | Integers sent and received |
|---|---|---|
| a `cc` way `[7]`, then an `osc` way with `range: [0, 100]`, no `scale` | 0 to 127 (the highest maximum) | the value 64 is sent 64 on the CC and 50 on the `osc` way; 50 received on the `osc` way is the value 64 |
| a `cc` way `[7]`, then an `osc` way with `range: [0, 1000]`, no `scale` | 0 to 1000 (the `osc` way) | the value 500 is sent 500 on the `osc` way and 64 on the CC; 64 received on the CC is the value 504 |
| a `cc` way `[7]`, then an `osc` way without `range`, no `scale` | 0 to 127 (the CC alone counts) | the value 100 is sent 100 and 100 received is the value 100 on the `osc` way; 200 received on it is the value 200, not clamped |
| one `osc` way without `range`, no `scale`, `default: 5000` | none known | admitted; the value 5000 is sent 5000, and -3 received is the value -3 |
| one `osc` way without `range`, `scale: [0, 10]`, `default: 10` | 0 to 10 | the value 10 is sent 10; 42 received is the value 42, not clamped |
| one `osc` way without `range`, `scale: [0, 10]`, `default: 11` | 0 to 10 | refused: `BadValue` at the `default` |

### The direction

An `osc` way says its direction by `transmit` and `recognize` and by no other field, from the point of view of the device or the software described: `transmit`, it sends the message; `recognize`, it receives it. A way with neither works on both sides. Only `none` removes a side.

Only `absolute` and `none` are admitted. `relative` and `both` designate the relative controllers of MIDI 2.0 (section 22.3 of 0.7) and are refused on an `osc` way.

The nearest correspondence with the mask `ACCESS` of OSCQuery:

| `ACCESS` | What OSCQuery says of the value | `transmit` | `recognize` |
|---|---|---|---|
| 0 | no value is associated with the address | `none` | `none` |
| 1 | the value may only be retrieved | `absolute` or absent | `none` |
| 2 | the value may only be set | `none` | `absolute` or absent |
| 3 | the value may be retrieved and set | `absolute` or absent | `absolute` or absent |

The meanings differ. `ACCESS` says whether a client may read or set a value held at an address. The two fields of a way say who sends the message. A value that may be retrieved by a query is not for that a message the device sends of its own, and an address with no value may still receive a message: a way with no `control_value` argument that is recognised is such a case, and a way with `none` on both sides claims nothing.

### What a way claims

An `osc` way claims its `path`, compared exactly, on each side where it works, with its coordinates `in` and `slice`. The rule is the one rule of claims of section 5.3.3, with no channel. The arguments, constants included, do not separate two claims. `slice` bounds the value of the `control_value` argument, as it is on the wire.

A reader MUST refuse the later way, as `BadValue` at that way (`/cc_params/entries/N/ways/M`), and the rule MUST name the earlier way by the same form of pointer, within one control as between two. Two ways with no side in common, or whose contexts cannot hold together, are admitted.

A `path` claims nothing against a message of MIDI, nor against the `osc` field of a control, `commands[].osc` or `replies`.

**A file with `midi_of`.** Read alone, the file is judged on its own entries only: a way of the file whose `path` a way of its target claims is read, since the target is not in the file, and a coordinate of a dimension the file does not know separates nothing. Joined to its target (section 23 of 0.7), the claims are those of the joined descriptor. A route of the file replaces the route of the target at the same `osc` address (section 23.7), with its ways; two routes at two `osc` addresses that claim one `path` on one side, in contexts that can hold together, are refused.

On "Pip". Each line gives the `osc` ways of a descriptor of 0.8 that is otherwise valid. Where a line says `in`, the device declares the dimension `mode` with the values `a` and `b`. `V` stands for `[{"type": "int", "control_value": true}]`.

| Ways written | Verdict |
|---|---|
| two controls, each with path `/pip/level` and `args` `V` | refused: `BadValue` at `/cc_params/entries/1/ways/0` |
| two controls on `/pip/level`, the first with `recognize: "none"`, the second with `transmit: "none"` | admitted: no side in common |
| two controls on `/pip/level`, the first with `in: {"mode": "a"}`, the second with `in: {"mode": "b"}` | admitted: the contexts cannot hold together |
| two controls on `/pip/level` with `args` `V`, `slice: [0, 63]` and `slice: [63, 127]` | refused: `BadValue` at `/cc_params/entries/1/ways/0`; with `[64, 127]` for the second, admitted |
| one control, two ways on `/pip/level`, with `args` `V` and `[]` | refused: `BadValue` at `/cc_params/entries/0/ways/1` |
| two controls on `/pip/do`, with the constants `"start"` and `"stop"` | refused: `BadValue` at `/cc_params/entries/1/ways/0`: arguments do not separate |
| one control on `/pip/level`, and a control whose `osc` is `/pip/level` | admitted |
| paths `/pip/level` and `/pip/Level` | admitted: two addresses |

### What a receiver and a sender do with a message

**A receiver.** A message at the `path` of a way is the message of that way only when its arguments follow the list: their types, their number and the constants. The number is that of the list, but for a last argument with `count`, which appears from `low` to `high` times. The value of the control is that of the `control_value` argument. With no such argument, the message reaches the control and neither gives it a value nor changes it: a trigger. The receiver gives its caller the free arguments by their `name`, in order, each repetition included.

**A sender.** It sends the constants as written and the value of the control at its place. For each `name`, it takes the values from its caller: one, or from `low` to `high` with `count`. When a required value is not there, it passes to the next way and invents nothing. How the caller passes these values is the business of the tool.

### What a reader refuses

`W` stands for `/cc_params/entries/N/ways/M`, `A` for `W/args/K`.

A reader reports one fault. For a way of kind `osc` of a descriptor of 0.8, it is the first met in this order:

1. A key of the way that its version does not define, then the form of the fields that 0.7 defines on a way, in this order: `ctrlType`, `ctrlIndex`, `value_bits`, `range`, `transmit`, `recognize`, `nrpn_lsb_range`, `in`, `slice` (a value of another type, a word outside its list). Both are judged as in 0.7.
2. Rows 1 and 2 of the table.
3. The arguments, one after the other in the written order, the one entirely before the next: rows 3 to 7, applied to that argument. A second `control_value` and a `name` written twice are faults of the later argument. Whether an argument is the last is known from the length of the list, at its turn.
4. Rows 8 and 9.

Inside one step, and for one argument, the lines of the table are taken from top to bottom.

| | Fault | At | Kind | Seen by |
|---|---|---|---|---|
| 1 | `path` absent, then `args` absent | `W/path`, `W/args` | `Missing` | the schema (at `W`) and the reader |
| 2 | a `path` that is not a string, does not start with `/`, or holds a space or one of `? * [ ] { }` | `W/path` | `BadValue` | the schema and the reader |
| 2 | an `args` that is not a list | `W/args` | `BadValue` | the schema and the reader |
| 3 | an argument that is not an object | `A` | `BadValue` | the schema and the reader |
| 3 | a key that is none of `type`, `const`, `control_value`, `name`, `count` or a note | `A/<key>` | `UnknownField` | the schema (at `A`) and the reader |
| 3 | an argument with no `type` | `A/type` | `Missing` | the schema (at `A`) and the reader |
| 3 | a `type` outside `int` and `string` | `A/type` | `BadValue` | the schema and the reader |
| 3 | none, or more than one, of `const`, `control_value` and `name` | `A` | `BadValue` | the schema and the reader |
| 4 | a `const` that is not of the `type`, or an integer outside 32 bits | `A/const` | `BadValue` | the schema and the reader |
| 5 | a `control_value` that is not `true`, or on a `type` other than `int` | `A/control_value` | `BadValue` | the schema and the reader |
| 5 | a second `control_value` | `A/control_value`, the second | `BadValue` | the schema (at `W/args`) and the reader |
| 6 | a `name` that is not of the form | `A/name` | `BadValue` | the schema and the reader |
| 6 | a `name` written twice | `A/name`, the second | `BadValue` | the reader alone |
| 7 | `count` on an argument that is not free | `A/count` | `BadValue` | the schema (at `A`) and the reader |
| 7 | `count` on a free argument that is not the last | `A/count` | `BadValue` | the reader alone |
| 7 | a `count` that is not a list of two | `A/count` | `BadValue` | the schema and the reader |
| 7 | a `low` that is not an integer of 0 or more, then a `high` that is not an integer of 1 or more | `A/count/0`, `A/count/1` | `BadValue` | the schema and the reader |
| 7 | `low` above `high` | `A/count` | `BadValue` | the reader alone |
| 8 | `ctrlIndex`, then `value_bits`, then `nrpn_lsb_range` on an `osc` way | that field | `BadValue` | the schema and the reader |
| 8 | `transmit`, then `recognize`, `relative` or `both` | that field | `BadValue` | the schema and the reader |
| 9 | `range`, then `slice`, on a way with no `control_value` argument | that field | `BadValue` | the schema and the reader |
| 10 | `path`, then `args`, on a way of `cc_params` of another `ctrlType`; reported before the other faults of the fields of that kind | `W/path`, `W/args` | `BadValue` | the schema and the reader |
| 10 | `path`, then `args`, on a `sysex` way of a `params` entry | `/params/entries/N/ways/M/path` or `/args` | `UnknownField` | the schema (at the way) and the reader |
| 11 | an `osc` way in `params`; its own faults of rows 1 to 9 come first | `/params/entries/N/ways/M/ctrlType` | `BadValue` | the schema and the reader |
| 12 | an address claimed twice | the later way, `W` | `BadValue` | the reader alone |
| 13 | `ctrlType: "osc"` in a descriptor of 0.7 | `W/ctrlType`, or the same place under `/params` | `BadValue` | the schema of 0.7 and the reader |
| 13 | `path` or `args` on a way in a descriptor of 0.7; reported before `ctrlType` | `W/path`, `W/args`, or the same place under `/params` | `UnknownField` | the schema of 0.7 (at the way) and the reader |

A descriptor of 0.4 to 0.6 gets the fault of its version: one that holds `ways` is refused at `ways`, and `path` or `args` on a control is `UnknownField` at that field. A file with `midi_of` is judged on its own text, in its own version, before it is joined, as for any field.

The schema says that a text is refused, and may report several errors for one text. The kind and the address of a fault are those of the reader.

### Cases of form

On "Pip". Each line is the only way of the control `/cc_params/entries/0` of a descriptor of 0.8 that is otherwise valid, unless the line says another version. Every way has `"ctrlType": "osc"` unless the line writes another.

| Written beside `ctrlType` | Verdict |
|---|---|
| `"path": "/pip/go", "args": []` | admitted: a message with no argument |
| `"path": "/pip/note", "args": [{"type": "string", "const": "on"}, {"type": "int", "control_value": true}]` | admitted |
| `"path": "/pip/say", "args": [{"type": "string", "name": "word", "count": [0, 4]}]` | admitted |
| `"args": []` | refused: `Missing` at `W/path` |
| `"path": "pip/go", "args": []` | refused: `BadValue` at `W/path` |
| `"path": "/pip/*", "args": []` | refused: `BadValue` at `W/path` |
| `"path": "/pip/go", "args": "i"` | refused: `BadValue` at `W/args` |
| `"path": "/pip/go", "args": [{"type": "float", "name": "x"}]` | refused: `BadValue` at `W/args/0/type` |
| `"path": "/pip/go", "args": [{"type": "int"}]` | refused: `BadValue` at `W/args/0` |
| `"path": "/pip/go", "args": [{"type": "int", "const": 1, "name": "x"}]` | refused: `BadValue` at `W/args/0` |
| `"path": "/pip/go", "args": [{"type": "int", "const": "1"}]` | refused: `BadValue` at `W/args/0/const` |
| `"path": "/pip/go", "args": [{"type": "int", "control_value": true}, {"type": "int", "control_value": true}]` | refused: `BadValue` at `W/args/1/control_value` |
| `"path": "/pip/go", "args": [{"type": "int", "name": "x"}, {"type": "int", "name": "x"}]` | refused: `BadValue` at `W/args/1/name` |
| `"path": "/pip/go", "args": [{"type": "int", "name": "x", "count": [1, 2]}, {"type": "int", "name": "y"}]` | refused: `BadValue` at `W/args/0/count` |
| `"path": "/pip/go", "args": [{"type": "int", "name": "x", "count": [0, 0]}]` | refused: `BadValue` at `W/args/0/count/1` |
| `"path": "/pip/go", "args": [{"type": "int", "name": "x", "count": [3, 2]}]` | refused: `BadValue` at `W/args/0/count` |
| `"path": "/pip/go", "args": [], "ctrlIndex": [1]` | refused: `BadValue` at `W/ctrlIndex` |
| `"path": "/pip/go", "args": [], "recognize": "both"` | refused: `BadValue` at `W/recognize` |
| `"path": "/pip/go", "args": [], "range": [0, 1]` | refused: `BadValue` at `W/range` |
| `"path": "/pip/go", "args": [{"type": "int", "const": "1"}, {"name": "y"}]` | refused: `BadValue` at `W/args/0/const`: the first argument before the second |
| `"path": "/pip/go", "args": [{"type": "int", "name": "x", "count": [1, 2]}, {"type": "float", "name": "y"}]` | refused: `BadValue` at `W/args/0/count`: the first argument before the second |
| `"path": "/pip/go", "args": [{"type": "float", "name": "x", "unit": "Hz"}]` | refused: `UnknownField` at `W/args/0/unit`: the key before the `type` |
| `"path": "pip/go", "args": [], "recognize": "twice"` | refused: `BadValue` at `W/recognize`: the form of a field of 0.7 before `path` |
| `"ctrlType": "cc", "ctrlIndex": [7], "path": "/pip/go"` | refused: `BadValue` at `W/path` |
| `"path": "/pip/go", "args": []` in a descriptor of 0.7 | refused: `UnknownField` at `W/path` |
| nothing, in a descriptor of 0.7 | refused: `BadValue` at `W/ctrlType` |

### What a converter does

Each way of 0.7 is a way of 0.8 with the same meaning. A converter from 0.7 writes no `osc` way, and derives none from `commands[].osc`, `replies` or `midi_in`. A descriptor admitted in 0.7 stays admitted in 0.8, as far as this draft goes.

### The example: cybermidi

cybermidi is a software that carries MIDI between hosts in OSC messages: `dstroud/cybermidi` at revision `5d8bc68`, file `lib/mod.lua`, read on 2026-11-27. Only facts are cited here; no code is copied.

```json
{
  "$schema": "https://w3id.org/osd/0.8/device.schema.json",
  "device": {
    "name": "cybermidi",
    "osc_prefix": "/cybermidi",
    "kind": "software",
    "transport": { "kind": "osc", "port": 10111 }
  },
  "cc_params": {
    "entries": [
      {
        "osc": "/midi_message",
        "expects": "trigger",
        "ways": [
          { "ctrlType": "osc", "path": "/cybermidi_msg", "args": [{ "type": "int", "name": "byte", "count": [1, 3] }] }
        ]
      },
      {
        "osc": "/ping",
        "expects": "trigger",
        "ways": [
          { "ctrlType": "osc", "path": "/cybermidi_ping", "args": [{ "type": "string", "const": "ping" }] }
        ]
      },
      {
        "osc": "/register",
        "expects": "trigger",
        "ways": [
          {
            "ctrlType": "osc",
            "path": "/cybermidi_reg",
            "args": [
              { "type": "string", "const": "reg" },
              { "type": "string", "name": "ip" },
              { "type": "string", "name": "host_name" }
            ]
          }
        ]
      }
    ]
  },
  "sources": [
    {
      "type": "contributed",
      "author": "dstroud",
      "url": "https://github.com/dstroud/cybermidi/blob/5d8bc68/lib/mod.lua",
      "commit": "5d8bc68",
      "license": "GPL-3.0",
      "notes": "Facts read from the code. The licence is to be confirmed in the LICENSE file."
    }
  ]
}
```

Each way works on both sides: the software sends and receives each of the three messages.

| `path` | In `lib/mod.lua` | A caller that sends it gives | A caller that receives it gets |
|---|---|---|---|
| `/cybermidi_msg` | sent, lines 147–195; received, lines 203–205 | from 1 to 3 integers for `byte` | the values of `byte`, 1 to 3, in order |
| `/cybermidi_ping` | sent, line 400; received, line 208 | nothing: the constant `"ping"` is written | nothing: the control is reached |
| `/cybermidi_reg` | sent, line 213; received, lines 216–227 | a string for `ip` and one for `host_name`, after the constant `"reg"` | `ip`, then `host_name` |

- The types of the arguments are read from the Lua code. No message was captured.
- The licence is reported as GPL-3.0. It is to be confirmed in the `LICENSE` file of the repository before the proposal is a candidate.
- The `osc` addresses of the controls (`/midi_message`, `/ping`, `/register`) and the prefix are chosen for this draft. No role and no title is given to a control.

What the example does not say is listed under "Open points".

## What a reference does

The need: to describe an OSC message that a device or a software sends or receives, with its address and its arguments.

| Reference | What it does of this need | Read at its source for this draft |
|---|---|---|
| OSCQuery (`github.com/Vidvox/OSCQueryProposal`, `README.md`) | `FULL_PATH` is the full address of a node. `TYPE` is the OSC type tag string of the method, one character per argument, and the arrays of values and ranges follow it. `OVERLOADS` gives the other type tag strings a method answers. `RANGE` gives for each argument `MIN`, `MAX` or `VALS`, the list of the only values accepted. `ACCESS` is an integer mask from 0 to 3: no value, retrieve, set, both. No attribute read there repeats an argument a variable number of times. | Yes: branch `master`, on 2026-11-27. |
| OSC 1.0 (`opensoundcontrol.stanford.edu/spec-1_0.html`) | A message is an address pattern, a type tag string and the arguments. An address starts with `/`; the names in it exclude the space and `# * , / ? [ ] { }`; a pattern matches addresses with `?`, `*`, `[]` and `{}`. The required tags are `i`, `f`, `s` and `b`, with others listed as nonstandard. It declares no direction, no range and no meaning of an argument. | Yes: on 2026-11-27. |
| MIDI 2.0 Property Exchange | A controller is said by `ctrlType` and `ctrlIndex`, with `transmit` and `recognize` (M2-117-UM, as section 22.1 of 0.7 lists them). The properties listed there hold no OSC address and no OSC argument. | No. Taken from the text of 0.7. M2-117-UM and the other resources are to be read before the proposal is a candidate. |
| MIDNAM (`MIDINameDocument10.dtd`, `MIDIEvents10.dtd`) | It names MIDI messages and their values. Neither file declares an element or an attribute for OSC. | Yes: the two DTD files kept in `osd-formats/tests/data/`. |

Where the draft departs:

- **A list of typed objects, where OSCQuery writes the string `TYPE`.** Reason: structure lives in JSON structures that a schema checks, one meaning per field. A string of tags says the types only, and the constant, the value of the control and the name of an argument would stand in other arrays matched by position.
- **`transmit` and `recognize`, where OSCQuery writes the mask `ACCESS`.** Reason: the ways of a control already say their direction with these two fields, those of Property Exchange; they say who sends the message, and `ACCESS` says who may read or set a value.
- **A constant argument.** OSCQuery can narrow an argument to one value with `VALS`. Reason of `const`: it is part of the message, which a receiver compares and a sender writes with no choice.
- **A repeated argument**, with `count`, where `TYPE` is a string of a fixed number of arguments and other lengths are other strings in `OVERLOADS`. Reason: one message of a variable length is one way.
- **The names `path` and `args`**, where OSCQuery says `FULL_PATH` and `TYPE`. Reason: they are the names `forward` of 0.7 already uses for an OSC message.

## Open points

- **The bytes of the tunnel of cybermidi.** The draft does not say what MIDI meaning the integers of `/cybermidi_msg` carry, nor their extent.
- **The destination of the messages of cybermidi**: the broadcast of `/cybermidi_ping` on the subnet, and where each message is sent.
- **The reply of cybermidi**: that a `/cybermidi_reg` answers a `/cybermidi_ping`.
- **The other addresses cybermidi receives**, which it passes to the current script (lines 233–234).
- **The other types of OSC** (`f`, `b` and the others).
- **The characters of a `path` and of a string.** OSC 1.0 also excludes `#` and `,` from the names of an address, and writes strings in non-null ASCII. This draft refuses neither.
- **More values than `high`.** The draft does not say what a sender does when its caller gives more values than `count` admits, or a value that is not of the type.
- **The licence of cybermidi** is to be confirmed in its `LICENSE` file, and its types by a capture.
- **The references** marked as not read are to be read at their source.

## Objections

None yet.
