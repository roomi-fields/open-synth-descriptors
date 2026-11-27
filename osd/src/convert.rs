//! The converter chain: 0.4 → 0.5 → 0.6 → 0.7 as an ordered list of rules, one function per
//! rule. A later version adds a rule at the end of [`RULES`] and touches no other.

use crate::{
    convert_from_04, convert_same, firmware_targets_of, full_value_width, kind_name, mixes, read, tighten_writings, CcEntry, Descriptor, Fault,
    FaultKind, Kind, SchemaUrl, Version, MESSAGE_FIELDS,
};
use crate::device::{known_scale, Way, PROFILE_OF_SOURCE};
use crate::json::Codec;
use crate::flat::{incomplete_of, is_complete};
use crate::providers::is_provider;
use crate::vocabulary::{convert_license, convert_source_type, PRODUCT_SOURCE_TYPES};
use serde_json::{Map, Value};
use std::collections::HashSet;

/// A step of the chain: the text of version `from` becomes the text of version `to`.
struct Rule {
    from: Version,
    to: Version,
    apply: fn(&str, Kind, &mut Chain) -> Result<String, Fault>,
}

/// What a rule knows of the whole conversion beyond the text it is given.
struct Chain {
    /// The version of the text given to the conversion.
    from: Version,
    /// The version the conversion ends in.
    target: Version,
}

/// The rules in order, each starting where the one before ends.
const RULES: &[Rule] = &[
    Rule { from: Version::V0_4, to: Version::V0_5, apply: rule_0_4_to_0_5 },
    Rule { from: Version::V0_5, to: Version::V0_6, apply: rule_0_5_to_0_6 },
    Rule { from: Version::V0_6, to: Version::V0_7, apply: rule_0_6_to_0_7 },
];

/// The text of version `from` carried to `target` (later than `from`) by the rules between them.
pub(crate) fn carry(text: &str, kind: Kind, from: Version, target: Version) -> Result<String, Fault> {
    let mut text = text.to_string();
    let mut chain = Chain { from, target };
    for rule in RULES.iter().filter(|r| r.from >= from && r.to <= target) {
        text = (rule.apply)(&text, kind, &mut chain)?;
    }
    Ok(text)
}

/// 0.4 to 0.5: structures for the strings, closed licences.
fn rule_0_4_to_0_5(text: &str, kind: Kind, _: &mut Chain) -> Result<String, Fault> {
    convert_from_04(read(text, kind)?, kind)
}

/// 0.5 to 0.6: the address changes, the writings 0.6 tightens are made, a firmware's lineage
/// becomes its one target. On the way to 0.7, a control of `cc_params` that mixes kinds of message
/// is first said by one way per kind (`mixed_controls_by_ways`); a conversion that ends in 0.6
/// refuses it.
fn rule_0_5_to_0_6(text: &str, kind: Kind, chain: &mut Chain) -> Result<String, Fault> {
    let mut v = same_form(text, kind, Version::V0_6)?;
    if kind == Kind::Firmware {
        firmware_targets_of(&mut v);
    }
    if kind == Kind::Device && chain.target >= Version::V0_7 {
        mixed_controls_by_ways(&mut v)?;
    }
    tighten_writings(&mut v)?;
    full_value_width(&mut v);
    let out = written(v);
    // The converter writes nothing its own reader refuses (a message claimed twice, across
    // controls or ways, `spec/0.6.md` "A reader MUST refuse the later claim of a message"): the
    // refusal is the reader's, at the same place, and no text is returned.
    read(&out, kind).map_err(|f| at_the_control(Fault { kind: FaultKind::NotConvertible, ..f }))?;
    Ok(out)
}

/// Says each control of `cc_params` whose flat fields mix kinds of message (proposal 0009) as 0.6
/// says a control reached by several messages: the first kind, in the order CC, NRPN, RPN, stays on
/// the entry, each other one is a way of `also` with the entry's `range`, the width of an NRPN or an
/// RPN being the 14 bits of the versions before 0.6. A way of `also` takes the `range` only when a
/// message of the mix is fully known: else the control keeps no way and its `range` stays on it. A
/// mix with an `nrpn_lsb_range` and no NRPN is refused at the control: no way holds it. The
/// `nrpn_lsb_range` stays on the entry, for the rule to 0.7 to place it
/// ([`carry_what_no_way_holds`]).
fn mixed_controls_by_ways(v: &mut Map<String, Value>) -> Result<(), Fault> {
    let Some(Value::Array(entries)) = v.get_mut("cc_params").and_then(|t| t.get_mut("entries")) else { return Ok(()) };
    for (n, entry) in entries.iter_mut().enumerate() {
        let Some(entry) = entry.as_object_mut().filter(|e| mixes(e)) else { continue };
        let [cc, nrpn, rpn] = MESSAGE_FIELDS.map(|keys| keys.map(|k| entry.contains_key(k)));
        if entry.contains_key("nrpn_lsb_range") && !nrpn.contains(&true) {
            let pointer = format!("/cc_params/entries/{n}");
            let rule = "the flat fields of the control mix kinds of message, and an nrpn_lsb_range is given without an NRPN: no way holds it".into();
            return Err(Fault { kind: FaultKind::NotConvertible, file: None, pointer, rule });
        }
        let way_kept = MESSAGE_FIELDS.iter().any(|keys| is_complete(entry, *keys));
        let mut also = Vec::new();
        let given = MESSAGE_FIELDS.iter().zip([cc, nrpn, rpn]).filter(|(_, given)| given.contains(&true));
        for (keys, _) in given.skip(1) {
            let mut way: Map<String, Value> = keys.iter().filter_map(|k| entry.remove(*k).map(|n| (k.to_string(), n))).collect();
            // The kind after the first of a mix is an NRPN or an RPN: it says its width.
            way.insert("value_bits".into(), Value::from(14));
            if let Some(range) = entry.get("range").filter(|_| way_kept) {
                way.insert("range".into(), range.clone());
            }
            also.push(Value::Object(way));
        }
        entry.insert("also".into(), Value::Array(also));
    }
    Ok(())
}

/// A fault the reader finds at a way of `also` that [`mixed_controls_by_ways`] wrote is the fault of
/// its control: the source, older than 0.6, has no `also`. The rule names the controls likewise.
fn at_the_control(mut f: Fault) -> Fault {
    if let Some(at) = f.pointer.find("/also/") {
        f.pointer.truncate(at);
    }
    let mut parts = f.rule.split("/also/");
    let mut rule = parts.next().unwrap_or_default().to_string();
    parts.for_each(|p| rule.push_str(p.trim_start_matches(|c: char| c.is_ascii_digit())));
    f.rule = rule;
    f
}

/// What the ways of `v`, the 0.7 form of the controls of `text` (0.6), do not take from the flat
/// entry (`spec/0.7.md`, what the converter writes); `controls` are those of `cc_params` as read.
/// The messages not fully known of a control become its note `_incomplete`, without the `value_bits`
/// the chain added to a text older than 0.6 (`from`, the version of the text given). A `range` beside
/// a non-empty `also` and no flat message goes to no way: when a way is left it is the note `_range`
/// ([`keep_range_as_note`]), else it stays on the entry. The `nrpn_lsb_range` of a control with ways
/// is on the way of the flat NRPN already; beside no flat NRPN field it goes to the single NRPN way,
/// and when no way can receive it, it stays on the entry as the note `_nrpn_lsb_range`.
fn carry_what_no_way_holds(text: &str, controls: &[CcEntry], v: &mut Map<String, Value>, from: Version) {
    let given: Value = serde_json::from_str(text).unwrap_or_default();
    let Some(given) = given.pointer("/cc_params/entries").and_then(Value::as_array) else { return };
    let Some(Value::Array(entries)) = v.get_mut("cc_params").and_then(|t| t.get_mut("entries")) else { return };
    for ((flat, control), entry) in given.iter().zip(controls).zip(entries) {
        let Some(entry) = entry.as_object_mut() else { continue };
        let in_also = flat.get("also").and_then(Value::as_array).is_some_and(|a| !a.is_empty());
        let on_entry = MESSAGE_FIELDS.iter().flatten().any(|k| flat.get(k).is_some());
        let way_kept = !control.ways.is_empty();
        if let Some(range) = flat.get("range").filter(|_| way_kept && in_also && !on_entry) {
            keep_range_as_note(entry, range, &control.ways);
        }
        let mut incomplete = flat.as_object().map(|f| incomplete_of(f, way_kept)).unwrap_or_default();
        if !incomplete.is_empty() {
            if from < Version::V0_6 {
                incomplete.iter_mut().for_each(|m| {
                    m.remove("value_bits");
                });
            }
            let note = free_key(entry, "_incomplete");
            entry.insert(note, incomplete.into_iter().map(Value::Object).collect());
        }
        let Some(block) = flat.get("nrpn_lsb_range") else { continue };
        let flat_nrpn = ["nrpn_msb", "nrpn_lsb"].iter().any(|k| flat.get(k).is_some());
        let Some(Value::Array(ways)) = entry.get_mut("ways") else { continue };
        if ways.is_empty() || ways.iter().any(|w| w.get("nrpn_lsb_range").is_some()) {
            continue;
        }
        let mut nrpn = ways.iter_mut().filter_map(Value::as_object_mut).filter(|w| w.get("ctrlType").is_some_and(|k| k == "nrpn"));
        match (nrpn.next(), nrpn.next()) {
            (Some(way), None) if !flat_nrpn => {
                way.insert("nrpn_lsb_range".into(), block.clone());
            }
            _ => {
                let note = free_key(entry, "_nrpn_lsb_range");
                entry.insert(note, block.clone());
            }
        }
    }
}

/// Keeps on `entry`, as the note `_range`, a `range` of 0.6 that none of its `ways` takes. The
/// entry's `default`, `values` and `step_count` are written in that range: with one of them, it is
/// the `scale` of the control when the ways alone give another reference scale. Else no `scale`.
fn keep_range_as_note(entry: &mut Map<String, Value>, range: &Value, ways: &[Way]) {
    entry.remove("scale");
    let measured = ["default", "values", "step_count"].iter().any(|k| entry.contains_key(*k));
    if measured && known_scale(None, ways).map(|s| Value::from(s.to_vec())).as_ref() != Some(range) {
        entry.insert("scale".into(), range.clone());
    }
    let note = free_key(entry, "_range");
    entry.insert(note, range.clone());
}

/// `key`, or, when `o` holds it, the first of `<key>_2`, `<key>_3`… it does not hold.
pub(crate) fn free_key(o: &Map<String, Value>, key: &str) -> String {
    let mut free = key.to_string();
    let mut n = 2;
    while o.contains_key(&free) {
        free = format!("{key}_{n}");
        n += 1;
    }
    free
}

/// 0.6 to 0.7: the address changes; the `_sources` of a device becomes `sources`, and its
/// `_midi_of_sources` `midi_of_sources`, each source carried to 0.7 ([`carry_source`]); the `_sources`
/// of another kind, free until then, is judged entry by entry ([`carry_notes_of_credit`]); the texts for
/// people a program read move to their fields; six of the seven fields 0.7 removes are dropped
/// (`transform_reverse`, `device.revision`, `transport.rate_limit_hz`, `sysex.footer`,
/// `midi_out.note_offset`, `identity.sends`) and the seventh, `sources[].imported_at`, is kept as
/// the note `_imported_at`; each port name of a device
/// or a profile becomes a rule. A text the reader of 0.7 refuses is not returned: the refusal is
/// `NotConvertible`, said of the text given ([`in_the_text_given`]). What no way of a control takes
/// from its flat entry is kept on it ([`carry_what_no_way_holds`]).
fn rule_0_6_to_0_7(text: &str, kind: Kind, chain: &mut Chain) -> Result<String, Fault> {
    let mut v = same_form(text, kind, Version::V0_7)?;
    if matches!(kind, Kind::Device | Kind::Profile) {
        port_names_to_rules(&mut v);
    }
    if kind == Kind::Device {
        let controls = carry_controls(text, &mut v)?;
        carry_what_no_way_holds(text, &controls, &mut v, chain.from);
        crate::device::move_manufacturer_id(&mut v);
        electra_to_id(&mut v);
        carry_credit(&mut v, "_sources", "sources");
        carry_credit(&mut v, "_midi_of_sources", "midi_of_sources");
        move_texts(&mut v);
    }
    if kind != Kind::Device {
        // A device's words went through the tables on the way to 0.5; the `_sources` of another
        // kind was a note until here.
        carry_notes_of_credit(&mut v, kind, chain.from)?;
    }
    let mut v = Value::Object(v);
    crate::device::drop_removed_in_0_7(&mut v);
    let Value::Object(mut v) = v else { unreachable!("a descriptor is a JSON object") };
    if kind == Kind::Device {
        drop_lua_capability_no_field_requires(&mut v, kind);
    }
    let out = written(v);
    read(&out, kind).map_err(|f| {
        let f = Fault { kind: FaultKind::NotConvertible, ..f };
        match kind {
            // A device and a profile alone move places a reader can refuse: the controls of a device
            // become ways and its credit changes key; a profile's port names become rules.
            Kind::Device | Kind::Profile => in_the_text_given(f, chain.from, text, &out),
            _ => f,
        }
    })?;
    Ok(out)
}

/// The credit under `old` takes the key `new`, at the same place, each source carried to 0.7. A
/// value that is no list is renamed as it is, for the reader to judge.
fn carry_credit(v: &mut Map<String, Value>, old: &str, new: &str) {
    if !v.contains_key(old) {
        return;
    }
    *v = crate::device::rename_key(v, old, new);
    if let Some(Value::Array(list)) = v.get_mut(new) {
        list.iter_mut().for_each(carry_source);
    }
}

/// The `_sources` of a maker profile, a sound, a fingerprint, a firmware or an assembly, free before
/// 0.7, and that of the assembly a sound was made on: each entry that is a source of 0.7 once carried
/// goes to `sources` ([`carry_note_of_credit`]). The `id` of a file are one set. From 0.4, a word of a
/// profile that the tables of 0.4 to 0.5 do not map is `NotConvertible` at its place.
fn carry_notes_of_credit(v: &mut Map<String, Value>, kind: Kind, from: Version) -> Result<(), Fault> {
    let from_0_4 = from == Version::V0_4;
    if from_0_4 && kind == Kind::Profile {
        refuse_words_without_a_table(v)?;
    }
    let mut ids = HashSet::new();
    carry_note_of_credit(v, from_0_4, &mut ids);
    if let Some(Value::Object(made_on)) = v.get_mut("made_on").filter(|_| kind == Kind::Sound) {
        carry_note_of_credit(made_on, from_0_4, &mut ids);
    }
    Ok(())
}

/// `_sources` of `v` as 0.7 says it, at the same place: `sources` holds, in their order, the entries
/// that read as sources of 0.7 once carried ([`credit_of_entry`]), each `id` once (`ids`: those
/// already written in the file). Every other entry is kept as it is, in a list under the note
/// `_sources_2` (the next free number when taken); a `_sources` that is no list is kept whole under
/// it. Nothing is written under `sources` without an entry for it.
fn carry_note_of_credit(v: &mut Map<String, Value>, from_0_4: bool, ids: &mut HashSet<String>) {
    let Some(note) = v.get("_sources") else { return };
    let (mut sources, mut kept) = (Vec::new(), Vec::new());
    let aside = match note {
        Value::Array(list) => {
            for entry in list {
                match credit_of_entry(entry, from_0_4).filter(|(_, id)| id.as_ref().is_none_or(|id| ids.insert(id.clone()))) {
                    Some((source, _)) => sources.push(source),
                    None => kept.push(entry.clone()),
                }
            }
            (!kept.is_empty()).then_some(Value::Array(kept))
        }
        whole => Some(whole.clone()),
    };
    // `v` still holds `_sources`: the key is the first numbered one it does not hold.
    let note_key = free_key(v, "_sources");
    for (key, value) in std::mem::take(v) {
        if key != "_sources" {
            v.insert(key, value);
            continue;
        }
        if !sources.is_empty() {
            v.insert("sources".into(), Value::Array(std::mem::take(&mut sources)));
        }
        if let Some(aside) = &aside {
            v.insert(note_key.clone(), aside.clone());
        }
    }
}

/// The source of 0.7 an entry of a free note of sources becomes, with its `id`; none when 0.7 refuses
/// it even carried. An object without `type` is `contributed`, `no type in 0.6` first in its `notes`.
fn credit_of_entry(entry: &Value, from_0_4: bool) -> Option<(Value, Option<String>)> {
    let mut source = entry.clone();
    let untyped = !source.as_object()?.contains_key("type");
    if from_0_4 {
        for (key, table) in WORDS_OF_0_4 {
            if let Some(Value::String(word)) = source.get_mut(key) {
                if let Ok(term) = table(word, "") {
                    *word = term;
                }
            }
        }
    }
    carry_source(&mut source);
    if untyped {
        let o = source.as_object_mut()?;
        o.insert("type".into(), Value::from("contributed"));
        first_line_of_notes(o, "no type in 0.6");
    }
    let id = crate::device::source_of_0_7(&source)?;
    Some((source, id))
}

type Table = fn(&str, &str) -> Result<String, Fault>;

/// The keys of a source whose words the tables of 0.4 to 0.5 map.
const WORDS_OF_0_4: [(&str, Table); 2] = [("type", convert_source_type), ("license", convert_license)];

/// A `type` or a `license` of a source of `_sources` that the tables of 0.4 to 0.5 do not map is
/// `NotConvertible` at its place. What is no text is left to the carrying.
fn refuse_words_without_a_table(v: &Map<String, Value>) -> Result<(), Fault> {
    let Some(Value::Array(list)) = v.get("_sources") else { return Ok(()) };
    for (n, source) in list.iter().enumerate() {
        for (key, table) in WORDS_OF_0_4 {
            if let Some(Value::String(word)) = source.get(key) {
                table(word, &format!("/_sources/{n}/{key}"))?;
            }
        }
    }
    Ok(())
}

/// `line` as the first line of the `notes` of a source. `notes` that is no text stays, for the reader
/// to judge.
fn first_line_of_notes(o: &mut Map<String, Value>, line: &str) {
    let notes = match o.get("notes") {
        None => line.to_string(),
        Some(Value::String(text)) => format!("{line}\n{text}"),
        Some(_) => return,
    };
    o.insert("notes".into(), Value::from(notes));
}

/// A fault the reader of 0.7 finds in `out`, the descriptor the converter wrote, at the pointer of the
/// text given, of version `from`: a way is a place of that text ([`place_of_way`]), in the pointer
/// and in the rule that names an earlier one; a named value is at its label; the credit is under
/// its `_` keys. Any other pointer is the same in both texts. `text` is the 0.6 text the last rule
/// was given: the text given itself when `from` is 0.6.
fn in_the_text_given(mut f: Fault, from: Version, text: &str, out: &str) -> Fault {
    let parse = |t: &str| serde_json::from_str::<Value>(t).unwrap_or_default();
    let parts: Vec<&str> = f.pointer.split('/').collect();
    let pointer = match parts[..] {
        ["", "cc_params", "entries", n, "ways", m, ..] => {
            let text = parse(text);
            if let Some((head, tail)) = f.rule.split_once(EARLIER_CONTROL) {
                let earlier = tail.split_once(':').and_then(|(place, rest)| place.split_once("/ways/").map(|(n, m)| (n, m, rest)));
                if let Some((n, m, rest)) = earlier {
                    f.rule = format!("{head}at {}:{rest}", place_of_way(&text, from, n, m));
                }
            }
            place_of_way(&text, from, n, m)
        }
        ["", table, "entries", n, "values", i, "value"] => {
            let out = parse(out);
            match out.pointer(&format!("/{table}/entries/{n}/values/{i}/title")).and_then(Value::as_str) {
                Some(label) => format!("/{table}/entries/{n}/values/{}", label.replace('~', "~0").replace('/', "~1")),
                None => return f,
            }
        }
        ["", "sources" | "midi_of_sources", ..] => format!("/_{}", &f.pointer[1..]),
        _ => return f,
    };
    f.pointer = pointer;
    f
}

/// How the rule of a message claimed twice names the earlier control.
const EARLIER_CONTROL: &str = "at /cc_params/entries/";

/// Where the text given writes the way `m` of the control `n` of `cc_params`. A text older than 0.6
/// has no `also`: the control. In a text of 0.6, the ways are those of the messages fully known, in
/// order: the flat one, which is the control, then those of `also`.
fn place_of_way(text: &Value, from: Version, n: &str, m: &str) -> String {
    let control = format!("/cc_params/entries/{n}");
    let Some(m) = m.parse::<usize>().ok().filter(|_| from >= Version::V0_6) else { return control };
    let Some(entry) = text.pointer(&control) else { return control };
    let known = |e: &Value| e.as_object().is_some_and(|e| MESSAGE_FIELDS.iter().any(|keys| is_complete(e, *keys)));
    let also = entry.get("also").and_then(Value::as_array).into_iter().flatten().enumerate();
    let flat = known(entry).then(|| control.clone());
    let place = flat.into_iter().chain(also.filter(|(_, a)| known(a)).map(|(k, _)| format!("{control}/also/{k}"))).nth(m);
    place.unwrap_or(control)
}

/// A plain name of `identity.ports` becomes a `contains` rule with no direction, platform or stack
/// (proposal 0004), in its order. An empty name recognises nothing and 0.7 refuses an empty value:
/// it is dropped, and a list of empty names stays, empty (dropping it would let a profile's names
/// back in).
fn port_names_to_rules(v: &mut Map<String, Value>) {
    let Some(Value::Array(ports)) = v.get_mut("identity").and_then(|i| i.get_mut("ports")) else { return };
    *ports = std::mem::take(ports)
        .into_iter()
        .filter(|name| name.as_str() != Some(""))
        .map(|name| serde_json::json!({"operator": "contains", "value": name}))
        .collect();
}

/// A word of `orientation` that is neither `centered` nor `0-based` has no `polarity`: the refusal of the
/// reader at the field is the converter's, as not convertible.
pub(crate) fn orientation_not_convertible(f: Fault) -> Fault {
    let entry = f.pointer.strip_prefix("/cc_params/entries/").and_then(|p| p.strip_suffix("/orientation"));
    match entry {
        Some(n) if n.bytes().all(|b| b.is_ascii_digit()) && !n.is_empty() => Fault { kind: FaultKind::NotConvertible, ..f },
        _ => f,
    }
}

/// The `_electra` of a command whose `parameter` is of type `sysex` with an integer `number` N
/// gives `id: N`, repeats kept. It goes only when it says nothing more, that is when it is exactly
/// `{"parameter": {"type": "sysex", "number": N}}`; with one more key it stays whole as a note.
/// Any other shape (`virtual` included) stays a note under `_electra`, with no `id`.
fn electra_to_id(v: &mut Map<String, Value>) {
    let Some(Value::Array(commands)) = v.get_mut("commands") else { return };
    for c in commands.iter_mut().filter_map(Value::as_object_mut) {
        let Some(electra) = c.get("_electra") else { continue };
        let Some(parameter) = electra.get("parameter").filter(|p| p.get("type").and_then(Value::as_str) == Some("sysex")) else { continue };
        let Some(number) = parameter.get("number").filter(|n| n.is_i64() || n.is_u64()).cloned() else { continue };
        if c.contains_key("id") {
            continue;
        }
        if *electra == serde_json::json!({"parameter": {"type": "sysex", "number": number}}) {
            c.remove("_electra");
        }
        c.insert("id".into(), number);
    }
}

/// A converted device's `capabilities` lists `lua` only while a field that remains requires it:
/// when the removals leave none, `lua` goes, and the key goes with it when the list is empty.
/// Nothing else of `capabilities` changes.
pub(crate) fn drop_lua_capability_no_field_requires(v: &mut Map<String, Value>, kind: Kind) {
    let has_lua = matches!(v.get("capabilities"), Some(Value::Array(l)) if l.iter().any(|c| c == "lua"));
    if !has_lua {
        return;
    }
    let mut probe = v.clone();
    probe.remove("capabilities");
    let required = match read(&written(probe), kind) {
        Ok(crate::Descriptor::Device(d)) => !d.derived_capabilities().is_empty(),
        _ => true,
    };
    if required {
        return;
    }
    let Some(Value::Array(list)) = v.remove("capabilities") else { return };
    let kept: Vec<Value> = list.into_iter().filter(|c| c != "lua").collect();
    if !kept.is_empty() {
        v.insert("capabilities".into(), Value::Array(kept));
    }
}

/// The controls of a device of 0.6 as 0.7 says them (`spec/0.7.md`, what the converter writes): the
/// reader of a text of 0.6 lifts each flat entry to its ways (`flat::lift`), and the entries it holds
/// replace those of the text, in the same order and with no control joined to another. A control
/// that gives no message fully known is kept, with no way. Returns the controls of `cc_params` it read.
fn carry_controls(text: &str, v: &mut Map<String, Value>) -> Result<Vec<CcEntry>, Fault> {
    let Descriptor::Device(d) = read(text, Kind::Device)? else { return Ok(Vec::new()) };
    let Value::Object(written) = crate::json::Object::write(&*d) else { return Ok(Vec::new()) };
    for table in ["cc_params", "params"] {
        let entries = written.get(table).and_then(|t| t.get("entries")).cloned();
        if let (Some(entries), Some(Value::Object(t))) = (entries, v.get_mut(table)) {
            t.insert("entries".into(), entries);
        }
    }
    Ok(d.cc_params.map(|t| t.entries).unwrap_or_default())
}

/// A source of 0.6 as 0.7 says it: its keys as the general fields, then who supplies it. A
/// `profile` that is no text, which 0.7 defines as one, goes under the note `_profile` (the next
/// free number when taken).
pub(crate) fn carry_source(source: &mut Value) {
    let Value::Object(o) = source else { return };
    general_fields(o);
    name_provider(o);
    if o.get(PROFILE_OF_SOURCE).is_some_and(|p| !p.is_string()) {
        let aside = free_key(o, "_profile");
        *o = crate::device::rename_key(o, PROFILE_OF_SOURCE, &aside);
    }
}

/// Proposal 0010: a `provider` that is not a well-formed identifier goes under `_provider`; a type
/// named after a product becomes `contributed`, with the partner's identifier unless the source
/// already names a provider, and the old word as the first line of `notes`.
fn name_provider(o: &mut Map<String, Value>) {
    if o.get("provider").is_some_and(|p| !p.as_str().is_some_and(is_provider)) {
        let aside = free_key(o, "_provider");
        *o = crate::device::rename_key(o, "provider", &aside);
    }
    let kind = o.get("type").and_then(Value::as_str);
    let Some(&(word, partner)) = PRODUCT_SOURCE_TYPES.iter().find(|(word, _)| Some(*word) == kind) else { return };
    o.insert("type".into(), Value::from("contributed"));
    if let Some(partner) = partner {
        o.entry("provider").or_insert_with(|| Value::from(partner));
    }
    first_line_of_notes(o, &format!("type in 0.6: {word}"));
}

/// `preset_name` and `name` become `title`, `revision` becomes
/// `version`, `content_sha256` becomes `sha256`, each only when its target is not already there
/// (the old key then stays); `preset_id` and `adaptation` drop when the last segment of the path
/// of `url` is exactly their text. `imported_at` becomes the note `_imported_at`, or the next free
/// suffix when that key is taken; every other key stays as it is.
fn general_fields(o: &mut Map<String, Value>) {
    if o.contains_key("imported_at") {
        let aside = free_key(o, "_imported_at");
        *o = crate::device::rename_key(o, "imported_at", &aside);
    }
    let ends_with = |key: &str| match (o.get("url"), o.get(key)) {
        (Some(Value::String(url)), Some(Value::String(id))) => !id.is_empty() && last_segment(url) == *id,
        _ => false,
    };
    let (drop_id, drop_adaptation) = (ends_with("preset_id"), ends_with("adaptation"));
    let has = |k: &str| o.contains_key(k);
    let (title, version, sha256) = (has("title"), has("version"), has("sha256"));
    let preset_name = has("preset_name");
    let out: Map<String, Value> = std::mem::take(o)
        .into_iter()
        .filter(|(k, _)| !(k == "preset_id" && drop_id || k == "adaptation" && drop_adaptation))
        .map(|(k, v)| {
            let to = match k.as_str() {
                "preset_name" if !title => "title",
                "name" if !title && !preset_name => "title",
                "revision" if !version => "version",
                "content_sha256" if !sha256 => "sha256",
                _ => return (k, v),
            };
            (to.to_string(), v)
        })
        .collect();
    *o = out;
}

/// The last segment of the path of `url`: the query, the fragment and a final slash do not count.
fn last_segment(url: &str) -> String {
    let url = url.split('#').next().unwrap_or_default().split('?').next().unwrap_or_default();
    let path = url.split_once("://").map_or(url, |(_, rest)| rest.split_once('/').map_or("", |(_, p)| p));
    path.trim_end_matches('/').rsplit('/').next().unwrap_or_default().to_string()
}

/// The `_` texts a program read in 0.6 become the fields of 0.7, on the entries of `cc_params` (a
/// control: `_usage`, `_name`, `_description`, `_notes`), of `params` and of `triggers` (`_name`,
/// `_description`). A text moves only when it is a string (not empty for `_name`) and the target
/// field is absent; otherwise the `_` key stays as it is, so nothing is lost and the result is valid.
fn move_texts(v: &mut Map<String, Value>) {
    const CONTROL: &[(&str, &str)] =
        &[("_usage", "value_description"), ("_name", "title"), ("_description", "description"), ("_notes", "notes")];
    const NAMED: &[(&str, &str)] = &[("_name", "title"), ("_description", "description")];
    for (section, moves) in [("cc_params", CONTROL), ("params", NAMED), ("triggers", NAMED)] {
        let Some(Value::Object(s)) = v.get_mut(section) else { continue };
        let Some(Value::Array(entries)) = s.get_mut("entries") else { continue };
        for entry in entries.iter_mut().filter_map(Value::as_object_mut) {
            for (from, to) in moves {
                let movable = match entry.get(*from) {
                    Some(Value::String(t)) => *from != "_name" || !t.is_empty(),
                    _ => false,
                };
                if movable && !entry.contains_key(*to) {
                    if let Some(t) = entry.remove(*from) {
                        entry.insert((*to).into(), t);
                    }
                }
            }
        }
    }
}

/// The document of `text` as it is (a repeated `capabilities` entry written once), its `$schema`
/// naming `to`.
fn same_form(text: &str, kind: Kind, to: Version) -> Result<Map<String, Value>, Fault> {
    let d = read(text, kind)?;
    let kept = convert_same(text, &d)?;
    let Ok(Value::Object(mut v)) = serde_json::from_str::<Value>(&kept) else { unreachable!("a descriptor is a JSON object") };
    v.insert("$schema".into(), SchemaUrl(kind_name(kind)).write(&to).unwrap_or_default());
    Ok(v)
}

fn written(v: Map<String, Value>) -> String {
    let mut out = serde_json::to_string_pretty(&Value::Object(v)).unwrap_or_default();
    out.push('\n');
    out
}
