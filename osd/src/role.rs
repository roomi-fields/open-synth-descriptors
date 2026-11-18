//! The role of a control (0.6): `{part, index?, setting}` on a CC, NRPN or RPN
//! entry and on a param entry. Each of `part` and `setting` is a term of the
//! state of the vocabulary the descriptor names in `role_vocabulary`, or an
//! extension `x-<registrant>-<term>`. Nothing here is written per role: a role
//! is checked against the state's terms.

use crate::json::{at_least, object, Opt, STRING};
use crate::role_vocabulary::{vocabulary_state, TermKind, Vocabulary};
use crate::vocabulary::is_extension;
use crate::{Descriptor, Device, Fault, FaultKind};

object! {
    /// What a control does, in the words every device shares: a part of the
    /// instrument, its number when there are several, and what is set.
    Role {
        "part" part: String = STRING,
        /// The number of the part when the instrument has several, from 1.
        "index" index: Option<i64> = Opt(at_least(1)),
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

impl Role {
    fn meaning(&self) -> RoleMeaning {
        if is_extension(&self.part) || is_extension(&self.setting) {
            RoleMeaning::NoKnownRole
        } else {
            RoleMeaning::Known
        }
    }
}

/// The roles of a device with their JSON pointers: the entries of `cc_params`,
/// then those of `params`.
pub(crate) fn device_roles(d: &Device) -> Vec<(String, &Role)> {
    let cc = d.cc_params.iter().flat_map(|t| &t.entries).enumerate().filter_map(|(n, e)| Some((format!("/cc_params/entries/{n}/role"), e.role.as_ref()?)));
    let params = d.params.iter().flat_map(|t| &t.entries).enumerate().filter_map(|(n, e)| Some((format!("/params/entries/{n}/role"), e.role.as_ref()?)));
    cc.chain(params).collect()
}

/// Each role of a descriptor, by its JSON pointer, with what it resolves to:
/// a known role, or no known role when it holds an extension. A descriptor of
/// another kind than a device has none.
pub fn roles(d: &Descriptor) -> Vec<(String, RoleMeaning)> {
    match d {
        Descriptor::Device(d) => device_roles(d).into_iter().map(|(p, r)| (p, r.meaning())).collect(),
        _ => Vec::new(),
    }
}

fn fault(kind: FaultKind, pointer: &str, rule: impl Into<String>) -> Fault {
    Fault { kind, file: None, pointer: pointer.to_string(), rule: rule.into() }
}

/// The term `name` of type `kind` in the state; none for an extension, which
/// stands in for a term the vocabulary lacks.
fn term_of<'v>(state: &'v Vocabulary, name: &str, kind: TermKind, pointer: &str) -> Result<Option<&'v crate::role_vocabulary::Term>, Fault> {
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
        let part = term_of(state, &role.part, TermKind::Part, &format!("{pointer}/part"))?;
        let setting = term_of(state, &role.setting, TermKind::Setting, &format!("{pointer}/setting"))?;
        if let (Some(p), Some(s)) = (part, setting) {
            if !s.applies_to.iter().flatten().any(|a| *a == p.name) {
                return Err(fault(FaultKind::BadValue, &pointer, format!("the setting {} does not apply to the part {}", s.name, p.name)));
            }
        }
    }
    Ok(())
}
