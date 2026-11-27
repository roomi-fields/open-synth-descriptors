# 0017 — MIDI and OSC on one device

## Status

- State: `draft`
- Candidate since: —
- Under the trial: —

## What it changes

The text and the schemas of 0.8, the version that follows 0.7: the key `device.transport` of the device descriptor takes a second form, a list of two transports, one MIDI and one OSC.

## Why

What this brings to the sharing of sounds on devices: a software that speaks both MIDI and OSC says both, so whoever receives its descriptor knows that either one reaches it, and by which one each message passes.

In 0.7 `device.transport` is one object with one `kind`, `midi` or `osc` ([device schema](../../schemas/0.7/device.schema.json)). A software that speaks both has to name one and leave the other unsaid, while its descriptor may already write both `frame` and `forward` in a command, and both `match_frame` and `match_osc` in a reply.

## The change

### The two forms

`device.transport` takes one of two forms:

- an object: one transport, as 0.7 writes it, with the same fields admitted;
- a list of exactly two objects, one of `kind` `midi` and one of `kind` `osc`.

The order of the list says nothing: both orders have the same meaning, and a reader writes the list back as it was written. A list of one element is refused: a list always says both.

| Where it is written | Fields admitted |
|---|---|
| the object form | `kind` (required), `host`, `port`, `reply_port`, `passthrough_prefix`, notes (keys that start with `_`) |
| the `osc` element of a list | `kind` (required), `host`, `port`, `reply_port`, `passthrough_prefix`, notes |
| the `midi` element of a list | `kind` (required), notes |

`host`, `port`, `reply_port` and `passthrough_prefix` say how the software is reached in OSC (section 5.3.8 of 0.7 says what each is, for OSC), so the `midi` element of a list does not hold them. The object form receives no new restriction: an object of `kind` `midi` that holds `host` is admitted, as in 0.7.

The draft adds no field. `rate_limit_hz`, which 0.7 removes from `transport` (section 5.3.2 of [0.7](../0.7.md)), stays removed. Every device descriptor that can write `transport` in 0.7 can write either form.

### The schema fragment

```json
{
  "$defs": {
    "device": {
      "properties": {
        "transport": {
          "if": { "type": "array" },
          "then": { "$ref": "#/$defs/transport_list" },
          "else": { "$ref": "#/$defs/transport" }
        }
      }
    },
    "transport": {
      "type": "object",
      "additionalProperties": false,
      "patternProperties": {
        "^_": {
          "description": "A note for people (provenance, caveats, usage); never read by a program."
        }
      },
      "required": ["kind"],
      "properties": {
        "kind": { "enum": ["midi", "osc"] },
        "host": { "type": "string" },
        "port": { "type": "integer" },
        "reply_port": { "type": "integer" },
        "passthrough_prefix": { "type": "string" }
      },
      "description": "How a software is reached; without it, by MIDI."
    },
    "transport_list": {
      "type": "array",
      "minItems": 2,
      "maxItems": 2,
      "items": {
        "$ref": "#/$defs/transport",
        "if": { "$ref": "#/$defs/midi_transport" },
        "then": {
          "properties": {
            "host": false,
            "port": false,
            "reply_port": false,
            "passthrough_prefix": false
          }
        }
      },
      "allOf": [
        {
          "if": {
            "prefixItems": [
              { "$ref": "#/$defs/midi_transport" },
              { "$ref": "#/$defs/midi_transport" }
            ]
          },
          "then": { "prefixItems": [true, { "properties": { "kind": false } }] }
        },
        {
          "if": {
            "prefixItems": [
              { "$ref": "#/$defs/osc_transport" },
              { "$ref": "#/$defs/osc_transport" }
            ]
          },
          "then": { "prefixItems": [true, { "properties": { "kind": false } }] }
        }
      ]
    },
    "midi_transport": {
      "type": "object",
      "required": ["kind"],
      "properties": { "kind": { "const": "midi" } }
    },
    "osc_transport": {
      "type": "object",
      "required": ["kind"],
      "properties": { "kind": { "const": "osc" } }
    }
  }
}
```

The fragment shows what changes in the device schema. The property `transport` of the definition `device` takes the place of the one of 0.7, and the four other definitions join the `$defs`. The descriptions of the five fields of a transport stay those of the schema of 0.7, left out of the fragment.

### An example

An invented software, "Loam", which speaks both:

```json
{
  "$schema": "https://w3id.org/osd/0.8/device.schema.json",
  "device": {
    "name": "Loam",
    "osc_prefix": "/loam",
    "kind": "software",
    "transport": [
      { "kind": "midi" },
      { "kind": "osc", "host": "127.0.0.1", "port": 9100, "reply_port": 9101, "passthrough_prefix": "loam" }
    ]
  },
  "commands": [
    { "osc": "/loam/start", "frame": [250], "forward": { "path": "/transport/start" } },
    { "osc": "/loam/save", "frame": [240, 125, 1, 247] },
    { "osc": "/loam/reload", "forward": { "path": "/project/reload" } }
  ],
  "replies": [
    { "match_frame": [240, 125, 2, 247], "match_osc": "/project/saved", "emit_osc": "/loam/saved" }
  ],
  "subscriptions": [
    { "on": "startup", "forward": { "path": "/project/listen" } }
  ]
}
```

| In this example | By which transport |
|---|---|
| `/loam/start` | one of the two: the byte 250 by MIDI, or `/transport/start` by OSC |
| `/loam/save` | MIDI |
| `/loam/reload` | OSC |
| the reply | the SysEx message by MIDI is this reply, and so is the OSC message `/project/saved` |
| the subscription | OSC |

### What a list says

A list says that the device, or the software, is reached by both MIDI and OSC.

### By which transport each message passes

This section holds for a descriptor whose `transport` is a list.

| What is written | Passes by |
|---|---|
| a frame (section 8.1 of 0.7), such as `commands[].frame` | MIDI |
| `replies[].match_frame` | MIDI |
| a way of a control whose `ctrlType` is a MIDI message: each of the eight words of section 5.3.3, `sysex` included | MIDI |
| the `forward` of a command or of a subscription | OSC |
| `replies[].match_osc` | OSC |
| `passthrough_prefix` | OSC |

A **sender** is whoever sends a message to the device. It is connected to the device by MIDI, by OSC, by both or by neither.

- **A command that writes both `frame` and `forward`** is carried out by one or the other. The sender sends only one of the two messages, never both for the same request, over a transport by which it is connected. Connected by both, the choice is its own: neither the order of the list nor the descriptor says which.
- **A command that writes only one of the two** passes by that one. A sender that is not connected by it does not send the command and invents nothing: it makes no frame out of a `forward`, and no OSC message out of a `frame`.
- **A reply that writes both `match_frame` and `match_osc`** is recognised on each one separately. A MIDI message that matches the first, or an OSC message that matches the second, is this reply, without waiting for the other.

The `script` of a command or of a reply is not a message and names no transport. This draft says neither by which transport passes a command that writes neither `frame` nor `forward`, nor by which passes an entry of `custom_commands`. The `script` and `custom_commands` are extensions (`x-osd-extension`) that a reader may ignore (section 5.3).

### What a list leaves unchanged

- The ways of a control keep their order, and a sender chooses among them as section 5.3.3 of 0.7 says.
- `commands[].osc` and the `osc` fields of the controls keep their meaning.
- `midi_in` keeps its meaning (section 9.3).
- `emit_osc` and `emit_args` keep their meaning: they say how a recognised message is reported.

This draft defines no way of a control that is a native OSC message. If a later change adds one, that change says how it passes under a list.

### With an object, or with no transport

With a `transport` that is an object, or with none, this draft adds no rule. In 0.7 `kind` decides: a command goes by its `frame` in `midi`, the way of a device without `transport`, and by its `forward` in `osc` (section 5.3.8 of 0.7). 0.7 does not define what is sent of a command that writes only the form of the other transport; this stays an open point.

### What a reader refuses

| Fault | Kind | At | What the schema reports |
|---|---|---|---|
| `transport` is neither an object nor a list | `BadValue` | `/device/transport` | the type, at the same place |
| a list whose length is not two | `BadValue` | `/device/transport` | the length, at the same place |
| an element that is not an object | `BadValue` | `/device/transport/N` | the type, at the same place |
| a key that is neither a field of a transport nor a note, `rate_limit_hz` included | `UnknownField` | the key: `/device/transport/N/<key>`, or `/device/transport/<key>` in the object form | the closed set of keys, at the object that holds the key |
| `kind` absent | `Missing` | `/device/transport/N/kind`, or `/device/transport/kind` | the required key, at the object that lacks it |
| a `kind` other than `midi` and `osc`, or a field of the wrong type | `BadValue` | the field | the closed list or the type, at the field |
| `host`, `port`, `reply_port` or `passthrough_prefix` on the `midi` element of a list | `BadValue` | the field | the forbidden property, at the field |
| two elements of the same `kind` | `BadValue` | `/device/transport/1/kind` | the forbidden property, at the same place |
| a list in a descriptor of 0.7 or earlier | `BadValue` | `/device/transport` | the type, at the same place |

The last line is what the readers of those versions report today: a reader of 0.7 or earlier reads `transport` as an object and refuses anything else (`osd/src/device.rs`, the field `transport` of `DeviceInfo`; `osd/src/json.rs`, the function `keys`).

The schema says that a text is refused, and may report several errors for one text. The kind and the address of a fault are those of the reader, which reports the first fault it meets in this order:

1. `transport` is neither an object nor a list.
2. The length of the list. A list of the wrong length is refused before any element is read.
3. Each element of a list, in the written order, the first one entirely before the second:
   1. it is not an object;
   2. an unknown key, the first one as written;
   3. `kind`: absent, then outside the two words;
   4. the type of `host`, `port`, `reply_port` and `passthrough_prefix`, in this order;
   5. on a `midi` element, the presence of one of these four, in the same order.
4. Two elements of the same `kind`.

An object is judged as 0.7 judges it: an unknown key, then `kind`, then the type of the other fields. `rate_limit_hz` in the object form stays a removed field, which a reader refuses as it does in 0.7; in an element of a list it is an unknown key of that element, judged at the turn of the element.

### Cases

On the invented software "Loam". Each line is the member of the block `device` of a descriptor of 0.8 that is otherwise valid, unless the line says another version.

| Written | Verdict |
|---|---|
| `"transport": [{"kind": "midi"}, {"kind": "osc", "host": "127.0.0.1", "port": 9100}]` | admitted |
| `"transport": [{"kind": "osc", "host": "127.0.0.1", "port": 9100}, {"kind": "midi"}]` | admitted: the same meaning as in the other order |
| `"transport": [{"kind": "midi", "_note": "by the virtual port"}, {"kind": "osc"}]` | admitted |
| `"transport": {"kind": "osc", "port": 9100}` | admitted: one transport |
| `"transport": {"kind": "midi", "host": "127.0.0.1"}` | admitted, as in 0.7 |
| no `transport` | admitted |
| `"transport": "osc"` | refused: `BadValue` at `/device/transport` |
| `"transport": null` | refused: `BadValue` at `/device/transport` |
| `"transport": []` | refused: `BadValue` at `/device/transport` |
| `"transport": [{"kind": "osc"}]` | refused: `BadValue` at `/device/transport` |
| `"transport": [{"kind": "midi"}, {"kind": "osc"}, {"kind": "osc"}]` | refused: `BadValue` at `/device/transport` |
| `"transport": [{"kind": "midi"}, "osc"]` | refused: `BadValue` at `/device/transport/1` |
| `"transport": [{}, {"kind": "osc"}]` | refused: `Missing` at `/device/transport/0/kind` |
| `"transport": [{"kind": "midi"}, {"kind": "serial"}]` | refused: `BadValue` at `/device/transport/1/kind` |
| `"transport": [{"kind": "midi"}, {"kind": "osc", "port": "9100"}]` | refused: `BadValue` at `/device/transport/1/port` |
| `"transport": [{"kind": "midi"}, {"kind": "osc", "path": "/loam"}]` | refused: `UnknownField` at `/device/transport/1/path` |
| `"transport": [{"kind": "midi", "rate_limit_hz": 100}, {"kind": "osc"}]` | refused: `UnknownField` at `/device/transport/0/rate_limit_hz` |
| `"transport": {"kind": "osc", "rate_limit_hz": 100}` | refused: `UnknownField` at `/device/transport/rate_limit_hz` |
| `"transport": [{"kind": "midi", "host": "127.0.0.1"}, {"kind": "osc"}]` | refused: `BadValue` at `/device/transport/0/host` |
| `"transport": [{"kind": "osc"}, {"kind": "midi", "port": 9100}]` | refused: `BadValue` at `/device/transport/1/port` |
| `"transport": [{"kind": "midi"}, {"kind": "midi"}]` | refused: `BadValue` at `/device/transport/1/kind` |
| `"transport": [{"kind": "osc"}, {"kind": "osc"}]` | refused: `BadValue` at `/device/transport/1/kind` |
| `"transport": [{"kind": "midi"}, {"kind": "osc"}]` in a descriptor of 0.7 | refused: `BadValue` at `/device/transport` |

### Resolution

At resolution `transport` follows section 23.4 of 0.7, unchanged. It is not an identity key: the result of a file with `midi_of` takes the whole value of its target, object or list, never joined with the one of the file. A target without `transport` gives a result without it.

An assembly keeps the `transport` of its stock, as sections 23.37 to 23.39 resolve it: a module adds its routes, and no transport.

Each file is read by the text of its own version (section 23.33): a file of 0.7 that writes a list is refused, even over a target of 0.8.

### The converters

The converter from 0.7 writes `device.transport` as it is:

- an object stays an object, with the same `kind` and the same fields;
- an absent `transport` stays absent;
- no list is written, whatever way, command or subscription speaks of the other transport.

The only removal of a field of `transport` stays that of `rate_limit_hz`, made by the conversion from 0.6 (section 5.3.2). As far as `transport` goes, a descriptor that is admitted in 0.7 is admitted in 0.8.

A converter to a format that does not carry `transport` names it among its losses, at `/device/transport`.

## What a reference does

The need: a device that is reached both by MIDI and by OSC.

| Reference | What it does of this need | Read at its source for this draft |
|---|---|---|
| `transport` of OSD 0.7 | One object, one `kind`, `midi` or `osc`, with `host`, `port`, `reply_port` and `passthrough_prefix`. It cannot name both. | Yes: the device schema of 0.7, the text of 0.7 (sections 5.3.2 and 5.3.8) and the reader (`osd/src/device.rs`). |
| OSCQuery (`HOST_INFO`) | Says how to reach the OSC server of a query server: `OSC_IP`, `OSC_PORT`, `OSC_TRANSPORT` (`TCP` or `UDP`), and `WS_IP` and `WS_PORT` for its WebSocket. None of the keys of `HOST_INFO` names a MIDI connection. The type tag `m` is MIDI data carried as an argument of an OSC message. | Yes: the README of `github.com/Vidvox/OSCQueryProposal`, branch `master`, on 2026-11-27. |
| MIDI 2.0 Property Exchange | Its messages travel as MIDI-CI System Exclusive, so a device it describes is reached by MIDI. No resource that declares an OSC address or port of the same device is known to this draft. | No. Written from knowledge of the specifications (M2-103-UM and the resource documents), to be read at their source before the proposal is a candidate. |
| MIDNAM (`MIDINameDocument10.dtd`, `MIDIEvents10.dtd`) | A document names a device by `Manufacturer` and `Model`, or by `Device`, and what it sends are MIDI messages. Neither file declares an element or an attribute for OSC, a host or a port. | Yes: the two DTD files kept in `osd-formats/tests/data/`. |

Where the draft departs:

- **Two forms of one key**, an object or a list, where one form per field is the simpler rule. Reason: compatibility. A `transport` of 0.7 reads without conversion, and a descriptor with one transport converts with its transport unchanged. A list required of everyone would rewrite every `transport` already written, to say nothing new.
- **From `transport` of 0.7**, which names one transport. Reason: a software that speaks both says both.
- **From OSCQuery**, whose `HOST_INFO` is asked of a running server and describes OSC and its WebSocket alone. Reason: a descriptor is read with no device in hand, and the need is to name MIDI beside OSC. The names `host` and `port` of 0.7 are kept rather than `OSC_IP` and `OSC_PORT`, and no field is added for TCP, UDP or a WebSocket.
- **From MIDI 2.0 Property Exchange and MIDNAM**, from which no field is taken. Reason: neither is known to name a second transport that is not MIDI (Property Exchange not read at its source, see above).

## Open points

- **A capture that only one side gives.** A reply with `match_frame` and `match_osc` is recognised on each separately. `emit_osc` or `emit_args` may use a capture that only one of the two provides: this draft does not say what is then reported when the other one is recognised.
- **An object, or no transport.** 0.7 does not define what is sent of a command that writes only the form of the other transport.
- **The references** marked as not read are to be read at their source.

## Objections

None yet.
