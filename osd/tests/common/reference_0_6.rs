//! What the 0.7 text of a driver or a maker profile must say of its 0.6 text:
//! every value, at every depth, as the correspondence of `spec/0.7.md` writes it
//! (5.3 to 5.3.7, "The credit of every kind" for the `_sources` of a profile,
//! 6.8.4, and 6.6 and 6.7 for the words of a profile given in 0.4).
//!
//! The correspondence is written here from the fields of the 0.6 text, without
//! the ways the reader makes of that text and without the functions of the
//! converter, which a defect shared with them would leave unseen.

use super::credit::address;
use osd::Kind;
use serde_json::{json, Map, Value};
use std::collections::{HashMap, HashSet};

type Object = Map<String, Value>;

/// One difference: the JSON pointer and what differs.
pub type Difference = (String, String);

/// What the 0.7 object must hold: by key, the value and the pointer that names
/// its change or its removal.
type Expected = Vec<(String, Value, String)>;

/// What the 0.7 document `new` does not say as the 0.6 document `old` does, one
/// difference by pointer. The entries of `cc_params` and `params` are compared
/// by [`entry`], the sources of a device by [`sources`] and those of a maker
/// profile by [`profile_credit`]; every other value is
/// compared at the deepest pointer of its 0.6 key, a value the correspondence
/// moves at the pointer of 0.6 it comes from, a key it does not give at its
/// pointer of 0.7, and a list whose length changes at the list. `given` is the
/// text `old` was converted from, whose entries are in the same order and whose
/// version says whether the words of a profile's sources go through the tables
/// of 0.4.
pub fn document(kind: Kind, given: &Value, old: &Value, new: &Value) -> Vec<Difference> {
    let mut out = Vec::new();
    let mut old = old.clone();
    drop_credit_of_no_entry(kind, &mut old, new);
    let old = &old;
    let (mut expected, moves) = correspondence(kind, old);
    let mut new = new.clone();
    // 5.3: the credit of a device and of a profile; the `_midi_of_sources` of a
    // profile stays a note, compared as every other value.
    match kind {
        Kind::Device => {
            for (old_key, new_key) in [("_sources", "sources"), ("_midi_of_sources", "midi_of_sources")] {
                out.extend(sources(old_key, old, &new));
                remove(&mut expected, "", old_key);
                remove(&mut new, "", new_key);
            }
        }
        Kind::Profile => {
            // A profile of 0.4 is the one that names no schema.
            let from_0_4 = given.get("$schema").is_none();
            let (differences, note) = profile_credit(old, &new, from_0_4);
            out.extend(differences);
            remove(&mut expected, "", "_sources");
            remove(&mut new, "", "sources");
            if let Some(note) = note {
                remove(&mut new, "", &note);
            }
        }
        _ => {}
    }
    Comparison { kind, given, moves, out: &mut out }.compare(&expected, &new, "");
    let mut named = HashSet::new();
    out.retain(|(pointer, _)| named.insert(pointer.clone()));
    out
}

/// A `_sources` of no entry credits nothing: where `new` writes no `sources` for it, it is taken
/// out of `old`, and nothing differs. So for the kinds whose `_sources` is free before 0.7, a sound
/// at its `made_on` too; the list of a device takes the key `sources` whatever it holds.
fn drop_credit_of_no_entry(kind: Kind, old: &mut Value, new: &Value) {
    let places: &[&str] = match kind {
        Kind::Device => &[],
        Kind::Sound => &["", "/made_on"],
        _ => &[""],
    };
    for at in places {
        let written = new.pointer(at).and_then(|o| o.get("sources")).is_some();
        let list = old.pointer(&pointer(at, "_sources")).and_then(Value::as_array);
        if !written && list.is_some_and(Vec::is_empty) {
            remove(old, at, "_sources");
        }
    }
}

/// 6.6 and 6.7: `source` with its `type` and its `license` written as 0.5 says
/// the words of 0.4; a word the tables do not rewrite stays.
fn words_of_0_5(mut source: Object) -> Object {
    const WORDS: [(&str, &str, &str); 4] = [
        ("type", "vendor", "vendor-doc"),
        ("type", "agent-contributed", "contributed"),
        ("type", "hardware-verified-partial", "hardware-verified"),
        ("license", "none", "NONE"),
    ];
    for (key, old, new) in WORDS {
        if source.get(key).is_some_and(|word| word == old) {
            source.insert(key.into(), new.into());
        }
    }
    source
}

/// What `new` does not say of the `_sources` of the 0.6 maker profile `old`, a
/// note of any value before 0.7, judged entry by entry: each entry that reads as
/// a source of 0.7 once carried, each `id` once, is under `sources` in its
/// order, the words of a profile given in 0.4 (`from_0_4`) going through the
/// tables; every other entry is kept as the text writes it, in its order, under
/// the note `_sources_2` (the first number free), and a `_sources` that is no
/// list is kept whole under it. No `sources` without an entry for it. A source
/// carried is named as in [`source`], an entry of the note that changes at its
/// place under `_sources`, and a list that changes length at `_sources`.
/// Returns the differences, and the key of the note when something is to be kept.
fn profile_credit(old: &Value, new: &Value, from_0_4: bool) -> (Vec<Difference>, Option<String>) {
    let (mut out, mut carried, mut kept) = (Vec::new(), Vec::new(), Vec::new());
    let mut ids = HashSet::new();
    let note = old.get("_sources");
    for (n, entry) in note.and_then(Value::as_array).into_iter().flatten().enumerate() {
        let (at, new_at) = (format!("/_sources/{n}"), format!("/sources/{}", carried.len()));
        let expected = entry.as_object().and_then(|given| {
            let worded = if from_0_4 { words_of_0_5(given.clone()) } else { given.clone() };
            let expected = untyped(source(&worded, &at, &new_at), &new_at);
            let written: Object = expected.iter().map(|(k, v, _)| (k.clone(), v.clone())).collect();
            let first_of_its_id = |id: &Value| ids.insert(id.to_string());
            (reads_as_a_source_of_0_7(&written) && written.get("id").is_none_or(first_of_its_id)).then_some(expected)
        });
        match expected {
            Some(expected) => carried.push(expected),
            None => kept.push((at, entry)),
        }
    }
    let written = new.get("sources");
    match written.and_then(Value::as_array) {
        Some(list) if !carried.is_empty() && list.len() == carried.len() => {
            for (n, (expected, found)) in carried.iter().zip(list).enumerate() {
                match found.as_object() {
                    Some(found) => out.extend(differences(expected, found, &format!("/sources/{n}"), &[])),
                    None => out.push(("/_sources".into(), format!("{found} is no source"))),
                }
            }
        }
        _ if carried.is_empty() && written.is_none() => {}
        _ => {
            let found = written.map_or("nothing".into(), Value::to_string);
            let why = format!("{} sources are to be written, 0.7 says {found}", carried.len());
            out.push(("/_sources".into(), why));
        }
    }
    let whole = note.filter(|note| !note.is_array());
    if kept.is_empty() && whole.is_none() {
        return (out, None);
    }
    // `old` holds `_sources`: the first free key is a numbered one.
    let key = old.as_object().map(|old| free_key(old, "_sources")).unwrap_or_default();
    let found = new.get(&key);
    let lost = |expected: String| {
        let found = found.map_or("nothing".into(), Value::to_string);
        ("/_sources".to_string(), format!("`{key}` is to be {expected}, 0.7 says {found}"))
    };
    match (whole, found.and_then(Value::as_array)) {
        (Some(whole), _) if found != Some(whole) => out.push(lost(whole.to_string())),
        (Some(_), _) => {}
        (None, Some(found)) if found.len() == kept.len() => {
            for ((at, entry), found) in kept.into_iter().zip(found) {
                if entry != found {
                    out.push((at, format!("{entry} is to be kept as it is, 0.7 says {found}")));
                }
            }
        }
        (None, _) => out.push(lost(format!("a list of {} entries", kept.len()))),
    }
    (out, Some(key))
}

/// A source of a profile that gives no `type` is `contributed`, `no type in 0.6`
/// the first line of its `notes`. `notes` that is no text stays.
fn untyped(mut expected: Expected, new_at: &str) -> Expected {
    const LINE: &str = "no type in 0.6";
    if expected.iter().any(|(k, _, _)| k == "type") {
        return expected;
    }
    expected.push(("type".into(), "contributed".into(), pointer(new_at, "type")));
    match expected.iter_mut().find(|(k, _, _)| k == "notes") {
        Some((_, notes, _)) => {
            if let Some(text) = notes.as_str() {
                *notes = format!("{LINE}\n{text}").into();
            }
        }
        None => expected.push(("notes".into(), LINE.into(), pointer(new_at, "notes"))),
    }
    expected
}

/// Whether the reader of 0.7 accepts `source` as a source of a maker profile.
fn reads_as_a_source_of_0_7(source: &Object) -> bool {
    let profile = json!({
        "$schema": address("0.7", Kind::Profile),
        "identity": { "reply_text": "acme" },
        "sources": [source],
    });
    osd::read(&profile.to_string(), Kind::Profile).is_ok()
}

/// Where a value of the expected document comes from in the 0.6 text.
struct Move {
    /// The pointer of 0.6 that names its change or its removal.
    from: String,
    /// Whether the value is named as one, whatever changes inside it.
    whole: bool,
}

/// The values the correspondence moves, by their pointer in the expected document.
type Moves = HashMap<String, Move>;

/// The 0.7 document the correspondence gives for the 0.6 document `old`, the
/// entries of `cc_params` and `params` and the sources left as 0.6 writes them,
/// and where the values it moves come from.
fn correspondence(kind: Kind, old: &Value) -> (Value, Moves) {
    let (mut doc, mut moves) = (old.clone(), Moves::new());
    // 5.4: the permanent address of the schema of the version.
    if let Some(schema) = doc.get_mut("$schema") {
        *schema = address("0.7", kind).into();
    }
    // 5.3.2: the removed fields outside the entries and the sources.
    remove(&mut doc, "/identity", "sends");
    port_rules(&mut doc, &mut moves);
    if kind == Kind::Device {
        remove(&mut doc, "/device", "revision");
        remove(&mut doc, "/device/transport", "rate_limit_hz");
        remove(&mut doc, "/sysex", "footer");
        remove(&mut doc, "/midi_out", "note_offset");
        lua_capability(old, &mut doc);
        manufacturer_id(&mut doc, &mut moves);
        command_ids(&mut doc, &mut moves);
        trigger_texts(&mut doc, &mut moves);
    }
    (doc, moves)
}

fn remove(doc: &mut Value, object: &str, key: &str) -> Option<Value> {
    doc.pointer_mut(object).and_then(Value::as_object_mut).and_then(|o| o.remove(key))
}

/// 5.3.7: each name of `identity.ports` is a `contains` rule, in its order, and an
/// empty name is dropped; `null` stays.
fn port_rules(doc: &mut Value, moves: &mut Moves) {
    let Some(Value::Array(ports)) = doc.pointer_mut("/identity/ports") else { return };
    let mut rules = Vec::new();
    for (n, name) in std::mem::take(ports).into_iter().enumerate() {
        if name == "" {
            continue;
        }
        let (at, from) = (format!("/identity/ports/{}", rules.len()), format!("/identity/ports/{n}"));
        for key in ["operator", "value"] {
            moves.insert(pointer(&at, key), Move { from: from.clone(), whole: true });
        }
        moves.insert(at, Move { from, whole: false });
        rules.push(json!({ "operator": "contains", "value": name }));
    }
    *ports = rules;
}

/// 5.3.2: `lua` leaves `capabilities`, and the key with it when the list becomes
/// empty, when `transform_reverse` was the last field holding Lua.
fn lua_capability(old: &Value, doc: &mut Value) {
    let reverse = entries_of(old).any(|e| e.get("transform_reverse").is_some());
    if !reverse || holds_lua_that_stays(old) {
        return;
    }
    let Some(Value::Array(list)) = doc.get_mut("capabilities") else { return };
    let before = list.len();
    list.retain(|c| c != "lua");
    if list.is_empty() && before > 0 {
        remove(doc, "", "capabilities");
    }
}

/// The entries of `cc_params` and of `params`.
fn entries_of(doc: &Value) -> impl Iterator<Item = &Value> {
    ["/cc_params/entries", "/params/entries"].into_iter().flat_map(|at| list(doc, at))
}

fn list<'a>(doc: &'a Value, at: &str) -> impl Iterator<Item = &'a Value> {
    doc.pointer(at).and_then(Value::as_array).into_iter().flatten()
}

/// Whether a field of the 0.6 device that 0.7 keeps holds Lua.
fn holds_lua_that_stays(old: &Value) -> bool {
    let at = |v: &Value, at: &str| v.pointer(at).is_some_and(|v| !v.is_null());
    let transfers = || list(old, "/sound_transfers");
    let elements = transfers().filter_map(|t| t.get("elements").and_then(Value::as_object)).flat_map(Map::values);
    let places = transfers().flat_map(|t| list(t, "/places"));
    entries_of(old).any(|e| at(e, "/transform"))
        || ["/commands", "/replies"].into_iter().any(|l| list(old, l).any(|x| at(x, "/script")))
        || list(old, "/custom_commands").next().is_some()
        || ["/identity/script", "/identity/firmware/script", "/programs/recall_script", "/programs/dump/script"]
            .into_iter()
            .any(|field| at(old, field))
        || old.pointer("/identity/firmware/decode").is_some_and(|d| d == "lua")
        || elements.chain(places).any(|x| at(x, "/script"))
}

/// 5.3.5: `device.manufacturer_id` is `identity.manufacturerId`, the block made
/// where there is none; a file with `midi_of` and no `identity` block keeps it as
/// the note `device._manufacturer_id`.
fn manufacturer_id(doc: &mut Value, moves: &mut Moves) {
    let Some(bytes) = remove(doc, "/device", "manufacturer_id") else { return };
    let from = || "/device/manufacturer_id".to_string();
    let linked = doc.get("midi_of").is_some();
    let has_identity = doc.get("identity").is_some_and(Value::is_object);
    if linked && !has_identity {
        doc["device"]["_manufacturer_id"] = bytes;
        moves.insert("/device/_manufacturer_id".into(), Move { from: from(), whole: true });
        return;
    }
    if !has_identity {
        doc["identity"] = json!({});
        moves.insert("/identity".into(), Move { from: from(), whole: false });
    }
    doc["identity"]["manufacturerId"] = bytes;
    moves.insert("/identity/manufacturerId".into(), Move { from: from(), whole: true });
}

/// 5.3.5: the `_electra` of a command whose `parameter` is of type `sysex` with an
/// integer `number` `N` gives its `id`; it goes only when it is exactly
/// `{"parameter": {"type": "sysex", "number": N}}`, and stays whole with one more
/// key. Any other shape stays, with no `id`.
fn command_ids(doc: &mut Value, moves: &mut Moves) {
    let Some(Value::Array(commands)) = doc.get_mut("commands") else { return };
    for (n, command) in commands.iter_mut().enumerate() {
        let Some(command) = command.as_object_mut() else { continue };
        let parameter = command.get("_electra").and_then(|e| e.get("parameter"));
        let number = parameter
            .filter(|p| p.get("type").is_some_and(|t| t == "sysex"))
            .and_then(|p| p.get("number"))
            .filter(|n| n.is_i64() || n.is_u64())
            .cloned();
        let Some(number) = number else { continue };
        let exact = json!({ "parameter": { "type": "sysex", "number": number.clone() } });
        if command.get("_electra") == Some(&exact) {
            command.remove("_electra");
        }
        command.insert("id".into(), number);
        let at = format!("/commands/{n}");
        moves.insert(format!("{at}/id"), Move { from: format!("{at}/_electra"), whole: true });
    }
}

/// 5.3.1: the `_name` and `_description` of a trigger are its `title` and
/// `description` when the text is a string (not empty for `_name`) and the field is
/// absent; else the `_` key stays.
fn trigger_texts(doc: &mut Value, moves: &mut Moves) {
    let Some(Value::Array(triggers)) = doc.pointer_mut("/triggers/entries") else { return };
    for (n, trigger) in triggers.iter_mut().enumerate() {
        let Some(trigger) = trigger.as_object_mut() else { continue };
        for key in ["_name", "_description"] {
            let Some(value) = trigger.get(key).cloned() else { continue };
            let (field, value) = moved("triggers", trigger, key, &value);
            if field == key {
                continue;
            }
            trigger.remove(key);
            let at = format!("/triggers/entries/{n}");
            moves.insert(pointer(&at, &field), Move { from: pointer(&at, key), whole: true });
            trigger.insert(field, value);
        }
    }
}

/// The comparison of the expected document with the 0.7 one, value by value.
struct Comparison<'a> {
    kind: Kind,
    given: &'a Value,
    moves: Moves,
    out: &'a mut Vec<Difference>,
}

impl Comparison<'_> {
    /// The pointer that names a change or a removal of what is expected at `at`.
    fn named(&self, at: &str) -> String {
        self.moves.get(at).map_or_else(|| at.to_string(), |m| m.from.clone())
    }

    fn differs(&mut self, at: &str, expected: &Value, found: Option<&Value>) {
        let found = found.map_or("nothing".into(), Value::to_string);
        self.out.push((self.named(at), format!("it is to be {expected}, 0.7 says {found}")));
    }

    fn compare(&mut self, expected: &Value, new: &Value, at: &str) {
        if self.moves.get(at).is_some_and(|m| m.whole) {
            if expected != new {
                self.differs(at, expected, Some(new));
            }
            return;
        }
        let entries = self.kind == Kind::Device && matches!(at, "/cc_params/entries" | "/params/entries");
        match (expected, new) {
            (Value::Object(expected), Value::Object(new)) => {
                for (key, value) in expected {
                    let at = pointer(at, key);
                    match new.get(key) {
                        Some(found) => self.compare(value, found, &at),
                        None => self.differs(&at, value, None),
                    }
                }
                for (key, value) in new {
                    if !expected.contains_key(key) {
                        let why = format!("{value} is written, which the 0.6 text does not give");
                        self.out.push((pointer(at, key), why));
                    }
                }
            }
            (Value::Array(expected), Value::Array(new)) if expected.len() == new.len() => {
                for (n, (value, found)) in expected.iter().zip(new).enumerate() {
                    let at = format!("{at}/{n}");
                    match (entries, value.as_object(), found.as_object()) {
                        (true, Some(old), Some(new)) => self.entry(&at, old, new),
                        _ => self.compare(value, found, &at),
                    }
                }
            }
            (Value::Array(expected), Value::Array(new)) => {
                let why = format!("{} elements became {}", expected.len(), new.len());
                self.out.push((self.named(at), why));
            }
            _ if expected != new => self.differs(at, expected, Some(new)),
            _ => {}
        }
    }

    /// One entry of `cc_params` or `params`, at `at`.
    fn entry(&mut self, at: &str, old: &Object, new: &Object) {
        let table = if at.starts_with("/params/") { "params" } else { "cc_params" };
        match self.given.pointer(at).and_then(Value::as_object) {
            Some(given) => self.out.extend(entry(table, at, given, old, new)),
            None => self.out.push((at.to_string(), "the text given holds no such entry".into())),
        }
    }
}

/// The entries of `table` (`cc_params` or `params`) of the 0.6 entry `old`,
/// compared with the 0.7 entry `new` at `at`. `given` is the entry as the text
/// converted writes it, in its own version: `old` itself for a text of 0.6.
pub fn entry(table: &str, at: &str, given: &Object, old: &Object, new: &Object) -> Vec<Difference> {
    let ways = if table == "params" { ways_of_param(old) } else { ways_of_control(old) };
    let partial = ways.is_empty();
    let mut expected = Expected::new();
    for (key, value) in old {
        // The numbers of the messages, and what a way takes from its entry.
        let on_the_ways = match table {
            "params" => matches!(key.as_str(), "pr" | "p" | "c" | "r" | "id"),
            _ => {
                matches!(key.as_str(), "cc" | "cc_lsb" | "nrpn_msb" | "nrpn_lsb" | "rpn_msb" | "rpn_lsb" | "value_bits" | "also")
                    || (!partial && key == "nrpn_lsb_range")
            }
        } || (!partial && matches!(key.as_str(), "range" | "transmit" | "recognize"));
        if on_the_ways || key == "transform_reverse" {
            continue;
        }
        let (new_key, value) = moved(table, old, key, value);
        expected.push((new_key, value, pointer(at, key)));
    }
    let incomplete = if table == "params" { Vec::new() } else { incomplete_of_control(given, old, partial) };
    if !incomplete.is_empty() {
        let key = free_key(old, "_incomplete");
        let named = pointer(at, &key);
        expected.push((key, incomplete.into(), named));
    }
    if let Some(block) = block_note(table, old, &ways) {
        let key = free_key(old, "_nrpn_lsb_range");
        let named = pointer(at, &key);
        expected.push((key, block, named));
    }
    if let Some(range) = range_note(table, old, &ways) {
        let key = free_key(old, "_range");
        let named = pointer(at, &key);
        expected.push((key, range, named));
    }
    if let Some(id) = negative_id_alone(old).filter(|_| table == "params") {
        let key = free_key(old, "_flat");
        let named = pointer(at, &key);
        expected.push((key, serde_json::json!({ "id": id }), named));
    }
    if let Some(scale) = scale(table, old, &ways) {
        expected.push(("scale".into(), scale, pointer(at, "scale")));
    }
    let mut out = differences(&expected, new, at, &["ways"]);
    let at = pointer(at, "ways");
    match new.get("ways").and_then(Value::as_array) {
        Some(written) if written.len() == ways.len() => {
            for (m, (way, written)) in ways.iter().zip(written).enumerate() {
                let at = format!("{at}/{m}");
                let Some(written) = written.as_object() else {
                    out.push((at, "a way is not an object".into()));
                    continue;
                };
                let way: Expected = way.iter().map(|(k, v)| (k.clone(), v.clone(), pointer(&at, k))).collect();
                out.extend(differences(&way, written, &at, &[]));
            }
        }
        written => out.push((at, format!("{} messages became {} ways", ways.len(), written.map_or(0, Vec::len)))),
    }
    out
}

/// The list of sources `key` of the 0.6 device (`_sources` or
/// `_midi_of_sources`), compared with the list of 0.7.
pub fn sources(key: &str, before: &Value, after: &Value) -> Vec<Difference> {
    let at = pointer("", key);
    let new_key = key.trim_start_matches('_');
    let (old, new) = (before.get(key), after.get(new_key));
    let (old, new) = match (old, new) {
        (Some(Value::Array(old)), Some(Value::Array(new))) if old.len() == new.len() => (old, new),
        (old, new) if old == new && !matches!(old, Some(Value::Array(_))) => return Vec::new(),
        (old, new) => {
            let count = |l: Option<&Value>| l.map_or("no list".into(), |l| l.as_array().map_or(l.to_string(), |l| format!("{} sources", l.len())));
            return vec![(at, format!("{} became {}", count(old), count(new)))];
        }
    };
    let mut out = Vec::new();
    for (n, (old, new)) in old.iter().zip(new).enumerate() {
        let (at, new_at) = (format!("{at}/{n}"), format!("/{new_key}/{n}"));
        match (old.as_object(), new.as_object()) {
            (Some(old), Some(new)) => out.extend(differences(&source(old, &at, &new_at), new, &new_at, &[])),
            _ if old == new => {}
            _ => out.push((at, format!("{old} became {new}"))),
        }
    }
    out
}

/// Where the keys of `expected` differ in `new`, each at its own pointer, and
/// the keys of `new` the correspondence does not give, at their pointer of 0.7.
fn differences(expected: &Expected, new: &Object, at: &str, apart: &[&str]) -> Vec<Difference> {
    let mut out = Vec::new();
    for (key, value, named) in expected {
        if new.get(key) != Some(value) {
            let found = new.get(key).map_or("nothing".into(), Value::to_string);
            out.push((named.clone(), format!("`{key}` is to be {value}, 0.7 says {found}")));
        }
    }
    for (key, value) in new {
        if !apart.contains(&key.as_str()) && !expected.iter().any(|(k, _, _)| k == key) {
            out.push((pointer(at, key), format!("{value} is written, which the 0.6 text does not give")));
        }
    }
    out
}

fn pointer(at: &str, key: &str) -> String {
    format!("{at}/{}", key.replace('~', "~0").replace('/', "~1"))
}

/// `key`, or the first of `<key>_2`, `<key>_3`… that `o` does not hold.
fn free_key(o: &Object, key: &str) -> String {
    let numbered = (2..).map(|n| format!("{key}_{n}"));
    std::iter::once(key.to_string()).chain(numbered).find(|k| !o.contains_key(k)).expect("a key is free")
}

/// The key and the value of 0.7 of a field the 0.7 entry keeps: a text for
/// people moved to its field (5.3.1), `values` as a list (5.3.3), `orientation`
/// as `polarity` (5.3.5), else the field itself.
fn moved(table: &str, old: &Object, key: &str, value: &Value) -> (String, Value) {
    let field = match (table, key) {
        (_, "_name") => Some("title"),
        (_, "_description") => Some("description"),
        ("cc_params", "_usage") => Some("value_description"),
        ("cc_params", "_notes") => Some("notes"),
        _ => None,
    };
    if let (Some(field), Some(text)) = (field, value.as_str()) {
        if (key != "_name" || !text.is_empty()) && !old.contains_key(field) {
            return (field.into(), value.clone());
        }
    }
    match (key, value) {
        ("values", Value::Object(named)) => {
            let list = named.iter().map(|(title, value)| serde_json::json!({ "value": value, "title": title }));
            (key.into(), list.collect())
        }
        ("orientation", _) if table == "cc_params" => match value.as_str() {
            Some("centered") => ("polarity".into(), "bipolar".into()),
            Some("0-based") => ("polarity".into(), "unipolar".into()),
            _ => (key.into(), value.clone()),
        },
        _ => (key.into(), value.clone()),
    }
}

/// The fields that give the numbers of a message.
const NUMBERS_OF_MESSAGES: [&str; 6] = ["cc", "cc_lsb", "nrpn_msb", "nrpn_lsb", "rpn_msb", "rpn_lsb"];

/// Whether the flat fields of a control, or an element of `also`, give a message, complete or not.
fn gives_a_message(given: &Object) -> bool {
    NUMBERS_OF_MESSAGES.iter().any(|k| given.contains_key(*k))
}

/// The way of the message the flat fields of a control, or of an element of `also`, give: its kind,
/// the numbers given and no other, its width. None when they give no message, or one not fully known:
/// an NRPN or an RPN of one number, a `cc_lsb` without its `cc` (5.3.3).
fn message(given: &Object) -> Option<Object> {
    let (kind, keys) = [("cc", ["cc", "cc_lsb"]), ("nrpn", ["nrpn_msb", "nrpn_lsb"]), ("rpn", ["rpn_msb", "rpn_lsb"])]
        .into_iter()
        .find(|(_, keys)| keys.iter().any(|k| given.contains_key(*k)))?;
    let complete = match kind {
        "cc" => given.contains_key("cc"),
        _ => keys.iter().all(|k| given.contains_key(*k)),
    };
    if !complete {
        return None;
    }
    let numbers: Vec<Value> = keys.iter().filter_map(|k| given.get(*k).cloned()).collect();
    let width = match kind {
        "cc" => (numbers.len() == 2).then(|| Value::from(14)),
        _ => given.get("value_bits").cloned(),
    };
    let mut way = Object::new();
    way.insert("ctrlType".into(), kind.into());
    way.insert("ctrlIndex".into(), numbers.into());
    way.extend(width.map(|w| ("value_bits".to_string(), w)));
    Some(way)
}

/// The elements of `also` of a control.
fn also(entry: &Object) -> impl Iterator<Item = &Object> {
    entry.get("also").and_then(Value::as_array).into_iter().flatten().filter_map(Value::as_object)
}

/// The note of the messages not fully known of a control, in the order of the source: the numbers,
/// the width and the `range` of the flat fields (the range only when the entry keeps a way, else it
/// stays on the entry), then each such element of `also` as it is (5.3.3). The
/// width is the one the text converted writes (`given`): a text older than 0.6
/// writes none, though its 0.6 text says one.
fn incomplete_of_control(given: &Object, entry: &Object, partial: bool) -> Vec<Value> {
    let unknown = |message_of: &Object| gives_a_message(message_of) && message(message_of).is_none();
    let flat = unknown(entry).then(|| {
        let width = |k: &str| k == "value_bits" && given.contains_key(k);
        let kept = |k: &str| NUMBERS_OF_MESSAGES.contains(&k) || width(k) || (!partial && k == "range");
        let fields: Object = entry.iter().filter(|(k, _)| kept(k)).map(|(k, v)| (k.clone(), v.clone())).collect();
        Value::from(fields)
    });
    flat.into_iter().chain(also(entry).filter(|a| unknown(a)).map(|a| Value::from(a.clone()))).collect()
}

/// The `transmit` and `recognize` of an entry, which each of its ways takes.
fn direction<'a>(entry: &'a Object) -> impl Iterator<Item = (String, Value)> + 'a {
    ["transmit", "recognize"].into_iter().filter_map(|k| entry.get(k).map(|v| (k.to_string(), v.clone())))
}

/// The ways of a control: the one of its flat fields, then each element of
/// `also` with its notes, each with its own `range`, a message not fully known
/// having none; the `nrpn_lsb_range` of the entry on the one NRPN way that
/// receives it (5.3.3).
fn ways_of_control(entry: &Object) -> Vec<Object> {
    let flat = message(entry);
    let has_flat = flat.is_some();
    let mut ways: Vec<Object> = flat
        .map(|way| (way, entry))
        .into_iter()
        .chain(also(entry).filter_map(|a| {
            let mut way = message(a)?;
            way.extend(a.iter().filter(|(k, _)| k.starts_with('_')).map(|(k, v)| (k.clone(), v.clone())));
            Some((way, a))
        }))
        .map(|(mut way, given)| {
            way.extend(given.get("range").map(|r| ("range".to_string(), r.clone())));
            way.extend(direction(entry));
            way
        })
        .collect();
    if let Some(block) = entry.get("nrpn_lsb_range") {
        let nrpn: Vec<usize> = (0..ways.len()).filter(|m| ways[*m]["ctrlType"] == "nrpn").collect();
        // A block beside a flat NRPN is that NRPN's: no other way takes it.
        let receiver = if entry.contains_key("nrpn_msb") || entry.contains_key("nrpn_lsb") {
            (has_flat && nrpn.first() == Some(&0)).then_some(0)
        } else {
            (nrpn.len() == 1).then(|| nrpn[0])
        };
        if let Some(m) = receiver {
            ways[m].insert("nrpn_lsb_range".into(), block.clone());
        }
    }
    ways
}

/// The one `sysex` way of an entry of `params`: `pr`, `p`, `c`, `r` as given, else
/// `id`, when it is 0 or more. Where the numbers alone do not say which fields
/// were given (an `id` beside them, numbers other than the first ones, or `pr`
/// alone, which reads as an `id`), the way keeps the fields in its note `_flat`.
fn ways_of_param(entry: &Object) -> Vec<Object> {
    const NUMBERS: [&str; 4] = ["pr", "p", "c", "r"];
    let given: Vec<&str> = NUMBERS.into_iter().filter(|k| entry.contains_key(*k)).collect();
    let index: Vec<Value> = match given.is_empty() {
        true => entry.get("id").filter(|_| negative_id_alone(entry).is_none()).cloned().into_iter().collect(),
        false => given.iter().map(|k| entry[*k].clone()).collect(),
    };
    if index.is_empty() {
        return Vec::new();
    }
    let mut way = Object::new();
    way.insert("ctrlType".into(), "sysex".into());
    way.insert("ctrlIndex".into(), index.into());
    if !given.is_empty() && (entry.contains_key("id") || given != NUMBERS[..given.len()] || given == ["pr"]) {
        let fields = given.iter().copied().chain(["id"]);
        let flat: Object = fields.filter_map(|k| entry.get(k).map(|v| (k.to_string(), v.clone()))).collect();
        way.insert("_flat".into(), flat.into());
    }
    way.extend(entry.get("range").map(|r| ("range".to_string(), r.clone())));
    way.extend(direction(entry));
    vec![way]
}

/// The `id` of an entry of `params` that no way can hold: negative, and the one
/// number the entry gives. The entry keeps it in its note `_flat` (5.3.3).
fn negative_id_alone(entry: &Object) -> Option<&Value> {
    let alone = !["pr", "p", "c", "r"].into_iter().any(|k| entry.contains_key(k));
    entry.get("id").filter(|id| alone && id.as_i64().is_some_and(|n| n < 0))
}

/// The `nrpn_lsb_range` the entry keeps as a note: it has ways and none
/// receives the block (5.3.3).
fn block_note(table: &str, entry: &Object, ways: &[Object]) -> Option<Value> {
    let taken = ways.iter().any(|w| w.contains_key("nrpn_lsb_range"));
    let block = entry.get("nrpn_lsb_range")?;
    (table == "cc_params" && !ways.is_empty() && !taken).then(|| block.clone())
}

/// The `range` the entry keeps as the note `_range`: a control with ways whose flat fields give no
/// message, complete or not, so no way takes it (5.3.3).
fn range_note(table: &str, entry: &Object, ways: &[Object]) -> Option<Value> {
    let range = entry.get("range")?;
    (table == "cc_params" && !ways.is_empty() && !gives_a_message(entry)).then(|| range.clone())
}

/// The `scale` of 0.7, only for a control with a `default`, `values` or `step_count` and a way
/// (5.3.3): for a `range` no way takes, that range where the most precise way, the first of the
/// highest maximum, has another; else, when the first message of the source is not fully known, its
/// range where the ways left give another; else the range of the first way, where another way has a
/// strictly higher maximum than the first.
fn scale(table: &str, entry: &Object, ways: &[Object]) -> Option<Value> {
    if !["default", "values", "step_count"].iter().any(|k| entry.contains_key(*k)) {
        return None;
    }
    let range = |w: &Object| -> Value {
        w.get("range").cloned().unwrap_or_else(|| {
            let wide = w.get("value_bits").is_some_and(|b| b == 14);
            serde_json::json!([0, if wide { 16383 } else { 127 }])
        })
    };
    let maximum = |w: &Object| range(w)[1].as_i64();
    if let Some(written_in) = range_note(table, entry, ways) {
        let highest = ways.iter().filter_map(maximum).max();
        let precise = ways.iter().find(|w| maximum(w) == highest).map(range)?;
        return (precise != written_in).then_some(written_in);
    }
    let (first, others) = ways.split_first()?;
    let first_given = std::iter::once(entry).chain(also(entry)).find(|g| gives_a_message(g))?;
    if message(first_given).is_none() {
        // The most precise way, the first on equal maxima.
        let most = others.iter().fold(first, |most, w| if maximum(w) > maximum(most) { w } else { most });
        return (range(first_given) != range(most)).then(|| range(first_given));
    }
    others.iter().any(|w| maximum(w) > maximum(first)).then(|| range(first))
}

/// What a source of 0.6 (at `at`) must be in 0.7 (at `new_at`): its keys as
/// 5.3 renames and drops them, the date of import and a `profile` that is no
/// string under notes, then who supplies it (6.8.4). A field changed is named at its 0.6 key; a field 0.6
/// does not have, at its pointer of 0.7.
fn source(old: &Object, at: &str, new_at: &str) -> Expected {
    let has = |k: &str| old.contains_key(k);
    // The product types of 0.6 and the provider each one gives.
    let product = old.get("type").and_then(Value::as_str).and_then(|word| match word {
        "electra-preset" => Some((word, Some("electra-one"))),
        "pencilresearch" | "knobkraft" => Some((word, Some(word))),
        "midnam" => Some((word, None)),
        _ => None,
    });
    // Notes taken: a key is free when neither the source nor an earlier note holds it.
    let mut taken = old.clone();
    let mut out = Expected::new();
    for (key, value) in old {
        let new_key = match key.as_str() {
            "preset_name" if !has("title") => "title".to_string(),
            "name" if !has("title") && !has("preset_name") => "title".to_string(),
            "revision" if !has("version") => "version".to_string(),
            "content_sha256" if !has("sha256") => "sha256".to_string(),
            "preset_id" | "adaptation" if ends_the_url(old, value) => continue,
            "imported_at" => free_key(&taken, "_imported_at"),
            "provider" if !value.as_str().is_some_and(well_formed) => free_key(&taken, "_provider"),
            "profile" if !value.is_string() => free_key(&taken, "_profile"),
            _ => key.clone(),
        };
        taken.insert(new_key.clone(), Value::Null);
        let value = match (key.as_str(), product) {
            ("type", Some(_)) => "contributed".into(),
            // Written with the line of the old type, below.
            ("notes", Some((word, _))) if value.is_string() => format!("type in 0.6: {word}\n{}", value.as_str().unwrap()).into(),
            _ => value.clone(),
        };
        out.push((new_key, value, pointer(at, key)));
    }
    let Some((word, provider)) = product else { return out };
    if !has("notes") {
        out.push(("notes".into(), format!("type in 0.6: {word}").into(), pointer(new_at, "notes")));
    }
    let given = old.get("provider").and_then(Value::as_str).is_some_and(well_formed);
    if let (Some(provider), false) = (provider, given) {
        out.push(("provider".into(), provider.into(), pointer(new_at, "provider")));
    }
    out
}

/// Whether `id` is exactly the last segment of the path of the `url` of the
/// source, the query, the fragment and a final `/` ignored.
fn ends_the_url(source: &Object, id: &Value) -> bool {
    let (Some(url), Some(id)) = (source.get("url").and_then(Value::as_str), id.as_str()) else { return false };
    let url = url.split(['?', '#']).next().unwrap_or_default();
    let path = match url.split_once("://") {
        Some((_, rest)) => rest.split_once('/').map_or("", |(_, path)| path),
        None => url,
    };
    !id.is_empty() && path.trim_end_matches('/').rsplit('/').next() == Some(id)
}

/// Whether `id` has the form of an identifier of a provider (6.8.1).
fn well_formed(id: &str) -> bool {
    let group = |g: &str| !g.is_empty() && g.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit());
    id.len() <= 64 && id.split('-').all(group)
}
