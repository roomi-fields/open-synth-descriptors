//! The converter chain: 0.4 → 0.5 → 0.6 → 0.7 as an ordered list of rules, one function per
//! rule. A later version adds a rule at the end of [`RULES`] and touches no other.

use crate::{
    convert_from_04, convert_same, firmware_targets_of, full_value_width, kind_name, mixes, read, tighten_writings, Descriptor, Fault,
    FaultKind, Kind, SchemaUrl, Version, MESSAGE_FIELDS,
};
use crate::json::Codec;
use crate::providers::is_provider;
use crate::vocabulary::PRODUCT_SOURCE_TYPES;
use serde_json::{Map, Value};

/// A step of the chain: the text of version `from` becomes the text of version `to`.
struct Rule {
    from: Version,
    to: Version,
    apply: fn(&str, Kind, &mut Chain) -> Result<String, Fault>,
}

/// What a rule knows of the whole conversion beyond the text it is given.
struct Chain {
    /// The version the conversion ends in.
    target: Version,
    /// The `nrpn_lsb_range` of each control of `cc_params` that mixes kinds of message, by its
    /// index: the 0.6 text between 0.5 and 0.7 cannot put it on a way of `also`.
    blocks: Vec<(usize, Value)>,
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
    let mut chain = Chain { target, blocks: Vec::new() };
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
        chain.blocks = mixed_controls_by_ways(&mut v)?;
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
/// RPN being the 14 bits of the versions before 0.6. A mix with a field no way holds without a
/// number invented or lost (an NRPN or an RPN given by one number, a `cc_lsb` without its `cc`, an
/// `nrpn_lsb_range` without an NRPN) is refused at the control. Returns the `nrpn_lsb_range` of
/// these controls, taken off their entries, for the NRPN way of 0.7.
fn mixed_controls_by_ways(v: &mut Map<String, Value>) -> Result<Vec<(usize, Value)>, Fault> {
    let mut blocks = Vec::new();
    let Some(Value::Array(entries)) = v.get_mut("cc_params").and_then(|t| t.get_mut("entries")) else { return Ok(blocks) };
    for (n, entry) in entries.iter_mut().enumerate() {
        let Some(entry) = entry.as_object_mut().filter(|e| mixes(e)) else { continue };
        let [cc, nrpn, rpn] = MESSAGE_FIELDS.map(|keys| keys.map(|k| entry.contains_key(k)));
        let why = if nrpn[0] != nrpn[1] || rpn[0] != rpn[1] {
            Some("an NRPN or an RPN is given by one number: a way says it by two")
        } else if cc == [false, true] {
            Some("a cc_lsb is given without its cc: no way says it")
        } else if entry.contains_key("nrpn_lsb_range") && !nrpn[0] {
            Some("an nrpn_lsb_range is given without an NRPN: no way holds it")
        } else {
            None
        };
        if let Some(why) = why {
            let pointer = format!("/cc_params/entries/{n}");
            let rule = format!("the flat fields of the control mix kinds of message, and {why}");
            return Err(Fault { kind: FaultKind::NotConvertible, file: None, pointer, rule });
        }
        if let Some(block) = entry.remove("nrpn_lsb_range") {
            blocks.push((n, block));
        }
        let mut also = Vec::new();
        let given = MESSAGE_FIELDS.iter().zip([cc, nrpn, rpn]).filter(|(_, given)| given.contains(&true));
        for (keys, _) in given.skip(1) {
            let mut way: Map<String, Value> = keys.iter().filter_map(|k| entry.remove(*k).map(|n| (k.to_string(), n))).collect();
            // The kind after the first of a mix is an NRPN or an RPN: it says its width.
            way.insert("value_bits".into(), Value::from(14));
            if let Some(range) = entry.get("range") {
                way.insert("range".into(), range.clone());
            }
            also.push(Value::Object(way));
        }
        entry.insert("also".into(), Value::Array(also));
    }
    Ok(blocks)
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

/// Puts the `nrpn_lsb_range` of each mixed control on its NRPN way.
fn place_blocks(v: &mut Map<String, Value>, blocks: &[(usize, Value)]) {
    let Some(Value::Array(entries)) = v.get_mut("cc_params").and_then(|t| t.get_mut("entries")) else { return };
    for (n, block) in blocks {
        let ways = entries.get_mut(*n).and_then(|e| e.get_mut("ways")).and_then(Value::as_array_mut);
        let way = ways.into_iter().flatten().filter_map(Value::as_object_mut).find(|w| w.get("ctrlType").is_some_and(|k| k == "nrpn"));
        if let Some(way) = way {
            way.insert("nrpn_lsb_range".into(), block.clone());
        }
    }
}

/// 0.6 to 0.7: the address changes; a device's `_sources` and `_midi_of_sources` become `sources`
/// and `midi_of_sources`, each source carried to 0.7 ([`carry_source`]); the texts for
/// people a program read move to their fields; the seven fields 0.7 removes are dropped
/// (`transform_reverse`, `device.revision`, `transport.rate_limit_hz`, `sysex.footer`,
/// `midi_out.note_offset`, `identity.sends`, `sources[].imported_at`); each port name of a device
/// or a profile becomes a rule.
fn rule_0_6_to_0_7(text: &str, kind: Kind, chain: &mut Chain) -> Result<String, Fault> {
    let mut v = same_form(text, kind, Version::V0_7)?;
    if matches!(kind, Kind::Device | Kind::Profile) {
        port_names_to_rules(&mut v);
    }
    if kind == Kind::Device {
        carry_controls(text, &mut v)?;
        place_blocks(&mut v, &chain.blocks);
        crate::device::move_manufacturer_id(&mut v);
        electra_to_id(&mut v);
        for (old, new) in [("_sources", "sources"), ("_midi_of_sources", "midi_of_sources")] {
            if v.contains_key(old) {
                let mut renamed = crate::device::rename_key(&v, old, new);
                if let Some(Value::Array(list)) = renamed.get_mut(new) {
                    list.iter_mut().for_each(carry_source);
                }
                v = renamed;
            }
        }
        move_texts(&mut v);
    }
    let mut v = Value::Object(v);
    crate::device::drop_removed_in_0_7(&mut v);
    let Value::Object(mut v) = v else { unreachable!("a descriptor is a JSON object") };
    if kind == Kind::Device {
        drop_lua_capability_no_field_requires(&mut v, kind);
    }
    Ok(written(v))
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

/// The `_electra` of a command whose `parameter` is `{"type": "sysex", "number": N}` with an integer `N`
/// becomes `id: N`, repeats kept. Any other shape (`virtual` included) stays a note under `_electra`.
fn electra_to_id(v: &mut Map<String, Value>) {
    let Some(Value::Array(commands)) = v.get_mut("commands") else { return };
    for c in commands.iter_mut().filter_map(Value::as_object_mut) {
        let number = c.get("_electra").and_then(|e| e.get("parameter")).filter(|p| p.get("type").and_then(Value::as_str) == Some("sysex")).and_then(|p| p.get("number")).filter(|n| n.is_i64() || n.is_u64()).cloned();
        if let (Some(n), false) = (number, c.contains_key("id")) {
            c.remove("_electra");
            c.insert("id".into(), n);
        }
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
/// that gives no message is kept, with no way.
fn carry_controls(text: &str, v: &mut Map<String, Value>) -> Result<(), Fault> {
    let Descriptor::Device(d) = read(text, Kind::Device)? else { return Ok(()) };
    let Value::Object(written) = crate::json::Object::write(&*d) else { return Ok(()) };
    for table in ["cc_params", "params"] {
        let entries = written.get(table).and_then(|t| t.get("entries")).cloned();
        if let (Some(entries), Some(Value::Object(t))) = (entries, v.get_mut(table)) {
            t.insert("entries".into(), entries);
        }
    }
    Ok(())
}

/// A source of 0.6 as 0.7 says it: its keys as the general fields, then who supplies it.
pub(crate) fn carry_source(source: &mut Value) {
    let Value::Object(o) = source else { return };
    general_fields(o);
    name_provider(o);
}

/// Proposal 0010: a `provider` that is not a well-formed identifier goes under `_provider`; a type
/// named after a product becomes `contributed`, with the partner's identifier unless the source
/// already names a provider, and the old word as the first line of `notes`.
fn name_provider(o: &mut Map<String, Value>) {
    if o.get("provider").is_some_and(|p| !p.as_str().is_some_and(is_provider)) {
        let mut aside = "_provider".to_string();
        let mut n = 2;
        while o.contains_key(&aside) {
            aside = format!("_provider_{n}");
            n += 1;
        }
        *o = crate::device::rename_key(o, "provider", &aside);
    }
    let kind = o.get("type").and_then(Value::as_str);
    let Some(&(word, partner)) = PRODUCT_SOURCE_TYPES.iter().find(|(word, _)| Some(*word) == kind) else { return };
    o.insert("type".into(), Value::from("contributed"));
    if let Some(partner) = partner {
        o.entry("provider").or_insert_with(|| Value::from(partner));
    }
    let line = format!("type in 0.6: {word}");
    let notes = match o.get("notes").and_then(Value::as_str) {
        Some(text) => format!("{line}\n{text}"),
        None => line,
    };
    o.insert("notes".into(), Value::from(notes));
}

/// `preset_name` and `name` become `title`, `revision` becomes
/// `version`, `content_sha256` becomes `sha256`, each only when its target is not already there
/// (the old key then stays); `preset_id` and `adaptation` drop when the last segment of the path
/// of `url` is exactly their text. `imported_at` is dropped; every other key stays as it is.
fn general_fields(o: &mut Map<String, Value>) {
    o.remove("imported_at");
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
