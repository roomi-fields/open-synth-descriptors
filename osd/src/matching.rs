//! The signs of identity a device and a fingerprint share (proposal 0004): each rule of a device
//! on a port name or on the endpoint name judged against a fingerprint, and two product instance
//! ids compared. What a detection makes of the verdicts is its own.

use crate::device::{PortDirection, PortOperator, PortRule};
use crate::{Device, Fingerprint};
use regex_syntax::hir::{ClassUnicode, ClassUnicodeRange};

/// What one rule says of one fingerprint.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MatchVerdict {
    /// The rule applies, and a name suits it.
    Matched,
    /// The rule applies, and no name suits it.
    NotMatched,
    /// The rule says nothing of this fingerprint: another system, or the names it reads are absent.
    NotApplied,
}

/// The verdict of the rule of a device at `pointer`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuleMatch {
    /// The JSON pointer of the rule in the device.
    pub pointer: String,
    pub verdict: MatchVerdict,
}

/// One verdict per rule of `device`: each entry of `identity.ports` in its order, then
/// `identity.endpoint_name`.
pub fn match_identity(device: &Device, fingerprint: &Fingerprint) -> Vec<RuleMatch> {
    let Some(identity) = &device.identity else { return Vec::new() };
    let ports = identity.ports.iter().flatten().flatten().enumerate();
    let mut out: Vec<RuleMatch> = ports
        .map(|(n, rule)| RuleMatch { pointer: format!("/identity/ports/{n}"), verdict: port_verdict(rule, fingerprint) })
        .collect();
    if let Some(Some(rule)) = &identity.endpoint_name {
        let verdict = match &fingerprint.endpoint_name {
            Some(name) => verdict(matches(rule.operator, &rule.value, name)),
            None => MatchVerdict::NotApplied,
        };
        out.push(RuleMatch { pointer: "/identity/endpoint_name".into(), verdict });
    }
    out
}

/// Whether the product instance ids of two fingerprints are the same text, case included; `None`
/// (unknown) when one of the two holds none or an empty one. No other field is read: whether the
/// two were taken from one unit is the detection's judgement.
pub fn same_product_instance_id(a: &Fingerprint, b: &Fingerprint) -> Option<bool> {
    Some(instance_id(a)? == instance_id(b)?)
}

fn instance_id(f: &Fingerprint) -> Option<&str> {
    f.product_instance_id.as_deref().filter(|id| !id.is_empty())
}

fn verdict(matched: bool) -> MatchVerdict {
    if matched { MatchVerdict::Matched } else { MatchVerdict::NotMatched }
}

fn port_verdict(rule: &PortRule, f: &Fingerprint) -> MatchVerdict {
    // A rule that names a platform or a stack applies only to a fingerprint that names the same.
    let other_platform = rule.platform.is_some_and(|p| f.platform != Some(p));
    let other_stack = rule.stack.is_some_and(|s| f.stack != Some(s));
    if other_platform || other_stack {
        return MatchVerdict::NotApplied;
    }
    let ports = f.ports.as_ref();
    let inputs = ports.and_then(|p| p.inputs.as_ref()).filter(|_| rule.direction != Some(PortDirection::Output));
    let outputs = ports.and_then(|p| p.outputs.as_ref()).filter(|_| rule.direction != Some(PortDirection::Input));
    if inputs.is_none() && outputs.is_none() {
        return MatchVerdict::NotApplied;
    }
    verdict(inputs.into_iter().chain(outputs).flatten().any(|name| matches(rule.operator, &rule.value, name)))
}

/// Whether `name` suits the rule, case ignored; an empty value matches nothing.
fn matches(operator: PortOperator, value: &str, name: &str) -> bool {
    if value.is_empty() {
        return false;
    }
    let (value, name) = (fold(value), fold(name));
    match operator {
        PortOperator::Equals => name == value,
        PortOperator::Contains => name.contains(&value),
        PortOperator::StartsWith => name.starts_with(&value),
        PortOperator::EndsWith => name.ends_with(&value),
    }
}

/// `text` under the simple case folding of Unicode, without normalisation: each character becomes
/// the first of the characters that fold as it does, so two texts that differ in case only fold to
/// one text.
fn fold(text: &str) -> String {
    text.chars()
        .map(|c| {
            let mut class = ClassUnicode::new([ClassUnicodeRange::new(c, c)]);
            class.case_fold_simple();
            class.ranges().first().map_or(c, ClassUnicodeRange::start)
        })
        .collect()
}
