# The process of proposals

**On trial.** This process is on trial on the next version of the format (0.7), with no adopter yet. A proposal accepted under the trial says so in its `Status` section. The owner validates this text, since it fixes a rule of the format.

## 1 What a proposal is

A proposal asks for a change of OSD: a new version of the format, a term of the vocabulary of roles, or the passage of an extension of [schemas/extensions.md](../schemas/extensions.md) to a term of a later state. One proposal is one file, `spec/proposals/<n>-<name>.md`, written from [the template](proposals/TEMPLATE.md). Each proposal has a number of its own, made of digits: a proposal takes the first free one when it enters main, and one already on main keeps its own. A published version never changes: a proposal accepted for a version enters the text of the next one, never the text of a published one. A correction that restates the text needs no proposal; a correction that states a meaning the text did not give is a proposal.

## 2 The states

One set of states serves every proposal, with no case of its own for a version, a term or an extension.

| State | Says |
|---|---|
| `draft` | written, open to change; decides nothing |
| `candidate` | frozen for review, open to objections; waits at least 14 days |
| `accepted` | decided by the owner; enters the next version, term or state |
| `rejected` | decided against by the owner, with its reason; kept |
| `withdrawn` | taken back by its author; kept |

A proposal goes `draft` to `candidate` to `accepted`. From `draft` or `candidate` it may go to `rejected` or `withdrawn`. A change of the text of a `candidate` puts it back to `draft`.

## 3 The delay

A `candidate` stays one at least **14 days**, counted from the day it enters `candidate`, before any acceptance. The day of entry is written in its `Status` section.

## 4 The condition of a reader

The owner's decision (Q6) joins to the delay at least one reader other than `osd` that passes the conformance suite. While no such reader exists (no adopter yet), the delay alone applies. Once one exists, an acceptance also requires it to pass the suite of the version the proposal changes.

This whole section is on trial on 0.7, like the rest of this text.

## 5 What the process takes from its references

The references were read on 2026-11-23 at the addresses below.

**OpenAPI** (github.com/OAI/OpenAPI-Specification: `CONTRIBUTING.md`, `proposals/2019-01-01-Proposal-Template.md`, the tags). A `proposals/` folder holds one file per proposal, copied from a template. The template's Status is "Proposal, Draft, Promoted, or Abandoned". A proposal is "reviewed and commented on or amended until it is either rejected or accepted". Versions have release candidates (tags 3.0.0-rc0 to rc2, 3.1.0-rc0 and rc1).

- What OSD takes: a file per proposal from a template, and a candidate before the final.
- Where it departs: other names of states (`candidate`, `accepted`, `rejected`, `withdrawn`) and a delay, since OSD has one owner and no review manager.

**TC39** (tc39.es/process-document). "There are six stages: a strawperson stage and five maturity stages" (0, 1, 2, 2.7, 3, 4). The committee approves each progression by consensus. "A proposal champion may request regressing a proposal to an earlier stage, or withdrawing it altogether". "The committee does not have an established concept of a rejected proposal". So TC39 states do not only advance, and it keeps no rejection.

- What OSD takes: a proposal may go back (a changed `candidate` returns to `draft`) and may be withdrawn by its author.
- Where it departs: five states instead of six stages, since a change of OSD is a text and not a feature to implement; and OSD has a `rejected` state, which TC39 has not, because one owner decides and his refusal with its reason is worth keeping.

**Python PEP** (peps.python.org/pep-0001). One file per PEP, PEP 12 as its template, and a header "Status: <Draft | Active | Accepted | Provisional | Deferred | Rejected | Withdrawn | Final | Superseded>". A rejected or withdrawn PEP stays as a record ("It is still important to have a record of this fact"). "The final authority for PEP approval is the Steering Council".

- What OSD takes: the file, the template, the `Status` section, and `rejected` and `withdrawn` kept.
- Where it departs: the owner decides, not a council; fewer states.

The delay of 14 days and its condition are the owner's (Q6), not a reference's.
