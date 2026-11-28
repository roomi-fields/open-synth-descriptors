//! What an export does not give back (docs/CADRE.md R8, docs/CADRE.md R16): the descriptor given and the one
//! the import of the file written gives back are compared field by field, as the JSON a reader
//! sees them, each control, command and way beside the one it became. A field is silent only
//! when it comes back equal; every other one is named, at its pointer or at a block that holds it.
//! No list says which fields a format loses: what the writer and the importer carry says it.

use crate::Loss;
use osd::{Descriptor, Device, Version};
use serde_json::{Map, Value};

/// A key as a JSON pointer segment.
pub(crate) fn segment(key: &str) -> String {
    key.replace('~', "~0").replace('/', "~1")
}

/// Whether `pointer` is `block` or lies under it.
fn under(pointer: &str, block: &str) -> bool {
    pointer.strip_prefix(block).is_some_and(|rest| rest.is_empty() || rest.starts_with('/'))
}

/// The JSON of a device in 0.7, the version the import returns, for a device before 0.7: it is
/// converted by the chain of `osd`, and one the chain refuses is written in the form of 0.7 as it
/// is typed.
fn converted(device: &Device) -> Value {
    let converted = crate::base::in_0_7(device.clone()).unwrap_or_else(|_| Device { schema: Some(Version::V0_7), ..device.clone() });
    written_once(converted)
}

/// Where what the writer wrote of a control comes back: its index in the descriptor given and in
/// the one the import gives, and the same for each of its ways that is written.
pub(crate) struct Control {
    pub given: usize,
    pub back: usize,
    pub ways: Vec<(usize, usize)>,
}

/// What comes back of the lists a format carries by item: each control, and each command by its
/// index in the descriptor given and in the one given back.
#[derive(Default)]
pub(crate) struct Became {
    pub controls: Vec<Control>,
    pub commands: Vec<(usize, usize)>,
}

/// Moves each item of the list at `pointer` of `back` to the index in `given` of what it was
/// written from, as `pairs` (index given, index back) say; an item no pair names comes after them,
/// and a place nothing comes back to is null.
fn align(given: &Value, back: &mut Value, pointer: &str, pairs: &[(usize, usize)]) {
    let length = given.pointer(pointer).and_then(Value::as_array).map_or(0, Vec::len);
    let Some(list) = back.pointer_mut(pointer).and_then(Value::as_array_mut) else { return };
    let mut items: Vec<Option<Value>> = std::mem::take(list).into_iter().map(Some).collect();
    let mut aligned = vec![Value::Null; length];
    for &(from, at) in pairs {
        if let (Some(place), Some(item)) = (aligned.get_mut(from), items.get_mut(at).and_then(Option::take)) {
            *place = item;
        }
    }
    aligned.extend(items.into_iter().flatten());
    *list = aligned;
}

fn written(device: &Device) -> Value {
    written_once(device.clone())
}

/// [`written`] for a device nothing reads again: it is not copied.
fn written_once(device: Device) -> Value {
    serde_json::from_str(&osd::write(&Descriptor::Device(Box::new(device)))).expect("a written device is JSON")
}

/// Where `place` stands in `given`: for each of its segments, the index of a list item or the rank
/// of a field in the order the object is written. A field the object does not hold comes after
/// those it holds, and nothing under it is ranked.
fn position(given: &Value, place: &str) -> Vec<usize> {
    let (mut at, mut ranks) = (Some(given), Vec::new());
    for part in place.split('/').skip(1) {
        let key = part.replace("~1", "/").replace("~0", "~");
        match at {
            Some(Value::Object(o)) => {
                ranks.push(o.keys().position(|k| *k == key).unwrap_or(o.len()));
                at = o.get(&key);
            }
            Some(Value::Array(a)) => {
                let item = key.parse::<usize>().unwrap_or(a.len());
                ranks.push(item);
                at = a.get(item);
            }
            _ => break,
        }
    }
    ranks
}

/// Puts the losses of an export in the order of the descriptor given (docs/INTERFACE.md,
/// `Translated`): list items by their index, a block before what lies under it, the fields of an
/// object in the order `osd` writes them. Losses of one place keep their order. `as_written` is the
/// JSON of `device` as `osd` writes it, when [`name_what_does_not_come_back`] gave it.
pub(crate) fn in_the_order_of(device: &Device, as_written: Option<Value>, losses: &mut [Loss]) {
    let given = as_written.unwrap_or_else(|| written(device));
    losses.sort_by_cached_key(|l| position(&given, &l.place));
}

/// The pointers where `back` is not `given`: a field missing, changed or added (`true`), at the
/// highest pointer that says it, in the order of the descriptor given. A list that does not come
/// back is said item by item when `said`, the losses of the writer, already names one of its items.
fn differences<'a>(given: Option<&'a Value>, back: Option<&'a Value>, at: String, said: &[Loss], out: &mut Vec<(String, bool)>) {
    // What comes back equal is silent, with all it holds: no pointer under it is built.
    if given == back {
        return;
    }
    let object = |o: Option<&'a Map<String, Value>>, k: &str| o.and_then(|o| o.get(k));
    match (given, back) {
        (Some(Value::Object(_)), Some(Value::Object(_))) | (Some(Value::Object(_)), None) if back.is_some() || at == "/cc_params" => {
            let (a, b) = (given.and_then(Value::as_object), back.and_then(Value::as_object));
            let added = b.into_iter().flat_map(Map::keys).filter(|k| object(a, k).is_none());
            for k in a.into_iter().flat_map(Map::keys).chain(added) {
                differences(object(a, k), object(b, k), format!("{at}/{}", segment(k)), said, out);
            }
        }
        (Some(Value::Array(a)), b) if matches!(b, Some(Value::Array(_))) || (b.is_none() && said.iter().any(|l| l.place != at && under(&l.place, &at))) => {
            let b = b.and_then(Value::as_array);
            for i in 0..a.len().max(b.map_or(0, Vec::len)) {
                differences(a.get(i), b.and_then(|b| b.get(i)), format!("{at}/{i}"), said, out);
            }
        }
        (a, _) => out.push((at, a.is_none())),
    }
}

/// Adds to `losses`, which hold what the writer says itself, each field of `device` that `back`,
/// the import of the file written, does not give back and that none of them holds. `became` says
/// which item of `back` each item written is; `why` says the reason at a pointer, `true` for a
/// field the import adds. Returns the JSON of `device` as `osd` writes it, when it is what was compared:
/// a device of 0.7 or later.
pub(crate) fn name_what_does_not_come_back(device: &Device, back: Option<Device>, became: &Became, losses: &mut Vec<Loss>, why: fn(&str, bool) -> String) -> Option<Value> {
    let as_it_is = device.schema >= Some(Version::V0_7);
    let given = if as_it_is { written(device) } else { converted(device) };
    let mut back = back.map_or(Value::Null, written_once);
    // The ways first, while each control is where the import put it.
    for c in &became.controls {
        let given = given.pointer(&format!("/cc_params/entries/{}", c.given));
        if let (Some(given), Some(back)) = (given, back.pointer_mut(&format!("/cc_params/entries/{}", c.back))) {
            align(given, back, "/ways", &c.ways);
        }
    }
    let controls: Vec<(usize, usize)> = became.controls.iter().map(|c| (c.given, c.back)).collect();
    align(&given, &mut back, "/cc_params/entries", &controls);
    align(&given, &mut back, "/commands", &became.commands);
    let mut found = Vec::new();
    differences(Some(&given), Some(&back), String::new(), losses, &mut found);
    for (place, added) in found {
        if !losses.iter().any(|l| under(&place, &l.place)) {
            let reason = if place.is_empty() { "the file written does not import back: nothing comes back".to_string() } else { why(&place, added) };
            losses.push(Loss { place, reason });
        }
    }
    as_it_is.then_some(given)
}
