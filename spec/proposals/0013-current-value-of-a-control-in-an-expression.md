# 0013 — The current value of a control in an expression

## Status

- State: `draft`
- Candidate since: —
- Under the trial: —

## What it changes

The text and the schemas of 0.8, the version that follows 0.7: an expression of `pre` (section 8.2 of [0.7](../0.7.md)) gains one leaf, `current`, the current value of a control that the description already has.

## Why

What this brings to the sharing of sounds on devices: a description says that a message carries what another setting holds now, so any host can offer such a gesture without a program of its own.

The need. A device with no mute message is muted by setting a level to zero. Unmuting has to send back the level set just before.

What a script does today. It writes into the message the current value of another control. In 0.7 the script is the Lua of `commands[].script` (a script that changes the message before it is sent) or of `custom_commands[].script` (a script that builds the message). The text of 0.7 does not say what such a script may read of the values of other controls.

What is missing for sharing. An expression reads only the arguments of its command. The `default` of a control is its value on an initialised sound, not its current value. So the gesture is written in Lua, which a reader without Lua does not run, and which depends on what its host hands the script.

## The change

### The form

```json
{ "current": "/params/entries/0", "otherwise": 100 }
```

`current` is a leaf of an expression, as an integer and `{"arg": name}` are. The schema:

```json
{
  "type": "object",
  "required": ["current", "otherwise"],
  "additionalProperties": false,
  "properties": {
    "current": { "type": "string", "pattern": "^/(cc_params|params)/entries/[0-9]+$" },
    "otherwise": { "type": "integer", "minimum": 0, "maximum": 127 }
  }
}
```

**Which control.** `current` is the JSON pointer (RFC 6901) of an entry of `cc_params` or of `params`, in the form that the `control` of a place of a sound transfer already has (section 12.1). It adds no address and no parameter. A reader MUST refuse, as `BadValue` at `current`, a pointer that names no entry of the description. A file with `midi_of` names an entry of the resolved description, where the entries of its target come first: a resolving reader judges the pointer there, and a reader that does not resolve the file keeps it unchecked.

**Its value**, written on the reference scale of the control, is the first of these that exists:

1. the last value the host sent or received for that control, by any of its ways, converted to the reference scale;
2. the `default` of the control;
3. `otherwise`.

Only a message of one of the ways of the control gives it a value. The frame of a command gives none, even when it changes the same setting in the device: so the mute below leaves the level the person had set. A device that sends that setting back by a way of the control does give it that value.

`otherwise` applies when the host holds no value and the control has no `default`. It is always written, so that an expression has a value in every description.

### With `{"arg": name}`, `pre` and the bounds

- `{"arg": name}` reads an argument of the command or a value bound before; `current` reads a control. The two are leaves of the same expressions.
- An expression stands in `pre` alone, so `current` is read there; a frame takes the result by the `arg` element that names the binding. No frame element reads a control.
- The value is taken when the message is built.
- The result is from 0 to 127, as every value of an expression. A reader MUST refuse, as `BadValue` at `current`, a control whose reference scale is known and does not lie within 0 to 127, and an `otherwise` outside 0 to 127 is refused by the schema.
- Where the reference scale is not known (an entry of `params` that gives none), the value is bounded to 0..127 when it is evaluated, as the result of `add` is, and it is not refused then. A value received outside a known scale is bounded likewise.

### A reader that holds no state of the device

A reader checks the leaf and gives it back; it needs no state to do so. A tool that builds a message and holds no value of the device has nothing at step 1: it takes the `default`, else `otherwise`. A tool SHOULD say that the value it sends is not one read from the device.

**Open point.** A recall of a program changes values that the host neither sent nor received one by one. Whether the host then forgets the values it holds is not said here. Recommendation: it forgets them at a program change it sends or receives, until it has read them again.

### With bit fields, if proposal 0011 is accepted too

This part holds only if [proposal 0011](0011-bit-fields-of-a-value-in-a-frame.md) is accepted.

- A binding whose `expr` is a `current` leaf alone, with no operation around it, keeps the whole value of the control. A frame writes it by fields, as an argument whose highest value is the upper bound of the reference scale; where that scale is not known, the value has no highest value stated.
- For such a binding, a reference scale of 0 or more that goes above 127 is not refused, and `otherwise` is an integer inside that scale (the schema bounds it from 0 to 2147483647, the reader checks the scale).
- No later expression reads such a binding when the scale of its control goes above 127 or is not known: an `{"arg": name}` that names it is `BadValue` at `arg`.
- Inside an operation, the value is bounded from 0 to 127 and the refusal above holds.

```json
"pre": [{ "bind": "cutoff", "expr": { "current": "/params/entries/1", "otherwise": 2048 } }],
"frame": [240, 125, 16, 2,
  { "arg": "cutoff", "bits": [11, 8] },
  { "arg": "cutoff", "bits": [7, 4] },
  { "arg": "cutoff", "bits": [3, 0] },
  247]
```

(`/params/entries/1` being a control whose reference scale is 0 to 4095.)

### With signed values, if proposal 0012 is accepted too

This part holds only if [proposal 0012](0012-signed-values.md) is accepted.

- A binding whose `expr` is a `current` leaf alone, with no operation around it, on an entry of `params` under `value_type` `s7` or `s14`, keeps the value, its sign and that type. A frame writes it as an argument of that type, whose `range` is the reference scale; where that scale is not known, as an argument without `range`.
- For such a binding, a reference scale inside the values of the type is not refused, below 0 or above 127 as it may go, and `otherwise` is an integer inside that scale (the schema bounds it from −8192 to 8191, the reader checks the scale). Where the scale is not known, the value and `otherwise` are bounded by the values of the type: a value outside them is bounded when it is evaluated, an `otherwise` outside them is `BadValue` at `otherwise`.
- No later expression reads such a binding: an `{"arg": name}` that names it is `BadValue` at `arg`.
- Inside an operation, nothing of this part applies: the rules above hold as for any control. A known scale that goes below 0 or above 127 is refused, `otherwise` is from 0 to 127, and a value of unknown scale is bounded from 0 to 127.
- With proposal 0011 accepted as well, the frame writes such a binding by fields of its two's complement, as a signed argument. Its `otherwise` keeps the bounds of this part; the schema bounds `otherwise` from −8192 to 2147483647, and the reader checks the scale and the type.

### Example

An invented device, "Orbe". Its messages start with `F0 7D 10` and end with `F7`; `7D` is the SysEx id kept for non-commercial use. The first parameter is the level of the voice, from 0 to 127. The command sends 0 when `mute` is 1, and the current level when `mute` is 0.

```json
{
  "params": {
    "get_frame": [240, 125, 16, 1, { "arg": "id" }, 247],
    "set_frame": [240, 125, 16, 3, { "arg": "id" }, { "arg": "value" }, 247],
    "value_type": "u7",
    "entries": [
      { "osc": "/voice/level", "ways": [{ "ctrlType": "sysex", "id": 4, "range": [0, 127] }] }
    ]
  },
  "commands": [
    {
      "osc": "/voice/mute",
      "args": [{ "name": "mute", "type": "bool" }],
      "pre": [{ "bind": "level", "expr":
        { "if": [{ "arg": "mute" }, 0,
                 { "current": "/params/entries/0", "otherwise": 100 }] } }],
      "frame": [240, 125, 16, 9, { "arg": "level" }, 247]
    }
  ]
}
```

After the level is set to 90, `mute` at 1 gives `F0 7D 10 09 00 F7`, then `mute` at 0 gives `F0 7D 10 09 5A F7`.

### What a reader and a converter do

A reader checks the leaf against the schema and the refusals above, and gives it back as written. A host that evaluates it finds the value by the three steps. No script and no `lua` capability is needed for this gesture.

A converter from 0.7 has nothing to rewrite: no descriptor of 0.7 holds the leaf. It turns no script into it. A converter that removes or reorders entries rewrites the pointers that name them; a converter to a format with no such leaf reports what it cannot carry.

### What stays a script

The owner decided that Lua stays, as the declared and opaque extension, for the rarer and more complex cases. For this need these stay a script:

- a memory of its own: a value kept aside and given back later, other than the one the control holds, as when the device sends the muted level back;
- a value that is no control of the description, or the value of a control chosen at run time;
- a value above 127 or below 0 used in a computation;
- asking the device for a value before the message is built;
- a sequence of messages whose content depends on the replies.

A script is needed there because `current` reads one named control and computes nothing more than the operations of 0.7.

## What a reference does

None of the three norms below was read at its source for this draft: each statement is taken over from an earlier exploration of the need, and is to be read again at the source before the proposal is a candidate.

- **MIDI 2.0 Property Exchange.** The controller list (M2-117-UM) says no dependency between controls. Its other resources were not checked.
- **MIDNAM.** It has no element for a current value.
- **OSCQuery.** It publishes the current value of a node to a client, with no rule that writes it into a message.

The form lets the sender of a message read that value.

## Tensions with the intentions of OSD

- **Universal, facilitator, semantic.** The description names the setting whose value it needs, and any host that follows the values of the controls can serve it.
- **No vendor field.** `current` names no maker and no product.
- **No loss.** A control whose known scale does not fit an expression is refused when read, not cut silently. Where the scale is unknown, the value is bounded, as an expression of 0.7 bounds a result.
- **One way to say one thing.** One leaf, in the expressions alone; no virtual parameter and no new address.
- **Structure in JSON.** A pointer of a form the format already checks by a pattern, and an integer; no string holds a mini-language.
- **Against the elements and expressions of 0.7.** One leaf more in the expressions; no frame element and no operation changes.
- **State.** The values of 0.7 expressions depend on the arguments alone. With this leaf, a message depends on what the host holds: two hosts may send different bytes for the same command. This is what the gesture asks, and the three steps say which value each sends.

## Objections

None yet.
