//! The role of a control (0.6): `{parts: [{part, index?}…], setting}` on a CC, NRPN or RPN
//! entry and on a param entry. A role is a path: one part or more, each with its number
//! when the instrument has several, then one setting, as the `paramPath` of MIDI-CI
//! Property Exchange (M2-117-UM). Each part and the setting is a term of the state of the
//! vocabulary the descriptor names in `role_vocabulary`, or an extension
//! `x-<registrant>-<term>`. Nothing here is written per role: a role is checked against
//! the state's terms (`stands_under`, `at_top`, `applies_to`).

use crate::json::{at_least, list_of, obj, object, Opt, Remarks, STRING};
use crate::role_vocabulary::{segment, vocabulary_state, Term, TermKind, Vocabulary};
use crate::vocabulary::is_extension;
use crate::{Descriptor, Device, Fault, FaultKind};

object! {
    /// A part of the instrument in a path, with its number when there are several.
    RolePart {
        "part" part: String = STRING,
        /// The number of the part when the instrument has several, from 1.
        "index" index: Option<i64> = Opt(at_least(1)),
    }
}

object! {
    /// What a control does, in the words every device shares: the parts from the
    /// outermost to the innermost, then what is set on the last.
    Role {
        "parts" parts: Vec<RolePart> = list_of(obj(), 1, usize::MAX),
        "setting" setting: String = STRING,
    }
}

/// What a role resolves to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RoleMeaning {
    /// Every term of the role is a term of the state the descriptor names.
    Known,
    /// The role holds an extension: no known role.
    NoKnownRole,
}

/// A role of a descriptor: where it is, what it resolves to, its `paramPath` and the role itself.
#[derive(Clone, Debug, PartialEq)]
pub struct RoleEntry {
    /// The JSON pointer of the role.
    pub pointer: String,
    pub meaning: RoleMeaning,
    /// The role as the `paramPath` of M2-117-UM: `/Filter/Envelope/AttackTime`.
    pub param_path: String,
    pub role: Role,
}

impl Role {
    fn meaning(&self) -> RoleMeaning {
        if self.parts.iter().any(|p| is_extension(&p.part)) || is_extension(&self.setting) {
            RoleMeaning::NoKnownRole
        } else {
            RoleMeaning::Known
        }
    }

    /// The `paramPath` of the role: each term as its CamelCase segment, the number of a part
    /// as a segment after it, the setting last; an extension is written as it is.
    fn param_path(&self) -> String {
        let seg = |t: &str| if is_extension(t) { t.to_string() } else { segment(t) };
        let mut out = String::new();
        for p in &self.parts {
            out.push('/');
            out.push_str(&seg(&p.part));
            if let Some(i) = p.index {
                out.push_str(&format!("/{i}"));
            }
        }
        out.push('/');
        out.push_str(&seg(&self.setting));
        out
    }
}

/// The roles of a device with their JSON pointers: the entries of `cc_params`,
/// then those of `params`.
pub(crate) fn device_roles(d: &Device) -> Vec<(String, &Role)> {
    let cc = d.cc_params.iter().flat_map(|t| &t.entries).enumerate().filter_map(|(n, e)| Some((format!("/cc_params/entries/{n}/role"), e.role.as_ref()?)));
    let params = d.params.iter().flat_map(|t| &t.entries).enumerate().filter_map(|(n, e)| Some((format!("/params/entries/{n}/role"), e.role.as_ref()?)));
    cc.chain(params).collect()
}

/// The pointer of each control that has a role, with the `paramPath` of that role, in the order of
/// the controls (`cc_params`, then `params`).
pub(crate) fn control_param_paths(d: &Device) -> Vec<(String, String)> {
    device_roles(d).into_iter().map(|(p, r)| (p.trim_end_matches("/role").to_string(), r.param_path())).collect()
}

/// Each role of a descriptor, by its JSON pointer, with what it resolves to (a known role,
/// or no known role when it holds an extension) and its `paramPath`. A descriptor of
/// another kind than a device has none.
pub fn roles(d: &Descriptor) -> Vec<RoleEntry> {
    match d {
        Descriptor::Device(d) => device_roles(d)
            .into_iter()
            .map(|(pointer, r)| RoleEntry { pointer, meaning: r.meaning(), param_path: r.param_path(), role: r.clone() })
            .collect(),
        _ => Vec::new(),
    }
}

fn fault(kind: FaultKind, pointer: &str, rule: impl Into<String>) -> Fault {
    Fault { kind, file: None, pointer: pointer.to_string(), rule: rule.into() }
}

/// The term `name` of type `kind` in the state; none for an extension, which
/// stands in for a term the vocabulary lacks.
fn term_of<'v>(state: &'v Vocabulary, name: &str, kind: TermKind, pointer: &str) -> Result<Option<&'v Term>, Fault> {
    if is_extension(name) {
        return Ok(None);
    }
    let what = if kind == TermKind::Part { "part" } else { "setting" };
    match state.term(name) {
        Some(t) if t.kind == kind => Ok(Some(t)),
        Some(_) => Err(fault(FaultKind::BadValue, pointer, format!("{name} is not a {what} of the vocabulary state {}", state.state))),
        None => Err(fault(
            FaultKind::BadValue,
            pointer,
            format!("{name} is no term of the vocabulary state {}: a {what} or an extension x-<registrant>-<term>", state.state),
        )),
    }
}

/// A role against the state: each term known, each part where it may stand, the setting on
/// the last part. An extension stands in for a term, so what it touches is not checked.
fn check_role(state: &Vocabulary, role: &Role, pointer: &str) -> Result<(), Fault> {
    let mut before: Option<Option<&Term>> = None;
    for (n, p) in role.parts.iter().enumerate() {
        let here = format!("{pointer}/parts/{n}");
        let term = term_of(state, &p.part, TermKind::Part, &format!("{here}/part"))?;
        if let Some(t) = term {
            match before {
                None if !t.opens_a_path() => {
                    return Err(fault(FaultKind::BadValue, &here, format!("{} does not open a path: it stands under {}", t.name, t.stands_under.iter().flatten().cloned().collect::<Vec<_>>().join(", "))));
                }
                Some(Some(up)) if !t.stands_under.iter().flatten().any(|u| *u == up.name) => {
                    return Err(fault(FaultKind::BadValue, &here, format!("{} does not stand under {}", t.name, up.name)));
                }
                _ => {}
            }
        }
        before = Some(term);
    }
    let setting = term_of(state, &role.setting, TermKind::Setting, &format!("{pointer}/setting"))?;
    if let (Some(Some(p)), Some(s)) = (before, setting) {
        if !s.applies_to.iter().flatten().any(|a| *a == p.name) {
            return Err(fault(FaultKind::BadValue, pointer, format!("the setting {} does not apply to the part {}", s.name, p.name)));
        }
    }
    Ok(())
}

/// The roles of a device against the state it names; before 0.6 neither
/// `role_vocabulary` nor a role is a field.
pub(crate) fn check_roles(d: &Device) -> Result<(), Fault> {
    let roles = device_roles(d);
    if d.schema < Some(crate::Version::V0_6) {
        if d.role_vocabulary.is_some() {
            return Err(fault(FaultKind::UnknownField, "/role_vocabulary", "role_vocabulary is not a field before 0.6"));
        }
        return match roles.first() {
            Some((p, _)) => Err(fault(FaultKind::UnknownField, p, "role is not a field before 0.6")),
            None => Ok(()),
        };
    }
    let state = match &d.role_vocabulary {
        Some(s) => Some(vocabulary_state(s).ok_or_else(|| {
            fault(FaultKind::BadValue, "/role_vocabulary", format!("a state of the vocabulary this reader holds: {}", crate::VOCABULARY_STATES.join(", ")))
        })?),
        None => None,
    };
    for (pointer, role) in roles {
        let state = state.ok_or_else(|| fault(FaultKind::Missing, "/role_vocabulary", "a descriptor with a role names the vocabulary state it was written with"))?;
        check_role(state, role, &pointer)?;
    }
    Ok(())
}

/// The index a `paramPath` segment of digits says, from 1; none for another segment.
fn index_of(segment: &str) -> Option<Result<i64, ()>> {
    (!segment.is_empty() && segment.chars().all(|c| c.is_ascii_digit())).then(|| segment.parse::<i64>().ok().filter(|i| *i >= 1 && !segment.starts_with('0')).ok_or(()))
}

/// The role a `paramPath` (M2-117-UM) says, read against the named state of the vocabulary:
/// a CamelCase segment is a known term, a segment of the form of an extension an extension,
/// a number the index of the part before it. A segment that is none of these is refused at
/// its place: the pointer is its position among the segments, from 0 (`/1`).
pub fn role_from_param_path(path: &str, state: &str) -> Result<Role, Fault> {
    let vocabulary = vocabulary_state(state).ok_or_else(|| {
        fault(FaultKind::BadValue, "", format!("a state of the vocabulary this reader holds: {}", crate::VOCABULARY_STATES.join(", ")))
    })?;
    let rest = path.strip_prefix('/').ok_or_else(|| fault(FaultKind::BadValue, "", "a path that starts with /"))?;
    let segments: Vec<&str> = rest.split('/').collect();
    let mut parts: Vec<RolePart> = Vec::new();
    let mut setting = None;
    for (k, seg) in segments.iter().enumerate() {
        let here = format!("/{k}");
        let last = k + 1 == segments.len();
        if let Some(index) = index_of(seg) {
            let index = index.map_err(|()| fault(FaultKind::BadValue, &here, "a number from 1"))?;
            match parts.last_mut() {
                Some(p) if p.index.is_none() && !last => p.index = Some(index),
                _ => return Err(fault(FaultKind::BadValue, &here, "a number is the index of the part before it, once, and not the last segment")),
            }
            continue;
        }
        let kind = if last { TermKind::Setting } else { TermKind::Part };
        let name = if is_extension(seg) {
            seg.to_string()
        } else {
            match vocabulary.terms.iter().find(|t| segment(&t.name) == *seg) {
                Some(t) if t.kind == kind => t.name.clone(),
                Some(t) => {
                    let what = if last { "setting" } else { "part" };
                    return Err(fault(FaultKind::BadValue, &here, format!("{seg} is the {} {}, where a {what} stands", if t.kind == TermKind::Part { "part" } else { "setting" }, t.name)));
                }
                None => {
                    return Err(fault(FaultKind::BadValue, &here, format!("{seg} is no CamelCase term of the vocabulary state {state}, no extension x-<registrant>-<term> and no number")))
                }
            }
        };
        if last {
            setting = Some(name);
        } else {
            parts.push(RolePart { part: name, index: None, remarks: Remarks::new() });
        }
    }
    match setting {
        Some(setting) if !parts.is_empty() => Ok(Role { parts, setting, remarks: Remarks::new() }),
        _ => Err(fault(FaultKind::BadValue, "", "a path of one part or more and a setting")),
    }
}
