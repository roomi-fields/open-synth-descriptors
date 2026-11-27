# 0010 — A source cites a public table of makers, firmwares and partners

## Status

- State: `draft`
- Candidate since: —
- Under the trial: —

Amended on 2026-11-25 with the precisions the building of 0.7 brought to `provider` at an import and at the conversion, and to the table. This change of a `candidate` text puts it back to `draft` ([PROCESS.md](../PROCESS.md), section 2): the delay of 14 days starts again the day it enters `candidate` again.

## What it changes

0.7: a source of `sources` gains one field, `provider`, that cites by identifier an entry of a public table (the table of providers) saying who supplies the source; the table is a data file of OSD, grown by contribution. It amends the text of 0.6, section 6.6 (the four source types named after a product). The single source type `partner-data`, which an earlier draft of proposal 0008 had for them, has been withdrawn: 0008 refers to this proposal, and the example of proposal 0009 already writes `contributed` with `provider`.

## Why

A descriptor must say where its facts come from: a maker, a community firmware, a partner's data. Today the only way to say it is a word of the closed list of `type`, and four of those words are the names of products. A standard that names products is not neutral: each new firmware or partner would need a new version of the format, and no reader could tell who supplied a source without knowing the word. An earlier draft of proposal 0008 removed the product names by folding the four into one term, `partner-data`, which lost who the partner is, kept only as a line of free text; that type has been withdrawn, and 0008 refers to this proposal.

With a table, the source cites an identifier, every tool reads the same provenance, the standard holds no name, and a new firmware or partner is a pull request on a data file, not a new version. This brings to the sharing of sounds on devices that anyone can say, in a form a program reads, who made what it relies on.

Decided by the owner on 2026-11-24:

- provenance belongs in the data structure: a source cites an identifier of a table, and the table, not the standard, holds the names;
- the table is a data file of OSD, updated by pull request, not a text of the norm; its growth is expected;
- the MIDI Association's manufacturer identifiers are the base for makers; an entry without one (a community firmware, a partner) is still admitted;
- the levels of trust in a source are SyntHub's, not OSD's;
- no name of a product, a brand or a partner enters the normative text or the closed lists of the schema; names appear in examples and in the table.

This proposal reads after proposal 0008. If 0008 changes the point below before it is accepted, the change is made in 0008 and this text takes it over.

## The change

### 1 The structure of a source

A source keeps `type`, the kind of evidence, with the terms of 0.6 section 6.6 for it (`hardware-verified`, `hardware-observed`, `software-verified`, `vendor-doc`, `vendor-osc-api`, `vendor-self-described`, `third-party-osc`, `community-re`, `contributed`, `test-fixture`), unchanged. The table replaces no term of evidence.

A source gains, beside `type`, an optional field `provider`: the identifier of an entry of the table of providers, saying who supplies the source.

- **Form of an identifier.** `^[a-z0-9]+(-[a-z0-9]+)*$`: lowercase letters and digits in groups joined by single hyphens (`electra-one`, `knobkraft`, `acme-synths`). At most 64 characters. The form is the only thing a schema checks about it; the schema holds no list of identifiers.
- **What a reader does.** A reader MUST refuse a `provider` that does not match the form, as `BadValue` at its pointer. A reader that meets a well-formed identifier absent from its version of the table MUST accept it, return it as it is, and give a warning that names the identifier; it MUST NOT refuse it. A reader MAY give the entry's name and address when its table has the entry.

The reason for accepting an unknown well-formed identifier is compatibility: the table grows by contribution, readers carry different versions of it, and a descriptor that cites an entry added last week must still read everywhere. The SPDX License List is read the same way (a reader is not required to know every identifier, and 0.6 section 6.7 already checks the shape of an SPDX id and never its presence in the registry). The departure from the closed lists of section 6.1, where a value outside the list is `BadValue`, is assumed: the table is not a closed vocabulary of the schema, it is a registry beside it, and its absence from a reader's copy is not a fault of the descriptor.

### 2 What an entry of the table holds

| Key | Holds |
|---|---|
| `id` | the identifier, of the form above; never reused |
| `name` | the name as its owner writes it |
| `kind` | one of a closed list: `maker`, `community-firmware`, `partner` |
| `midi_manufacturer_id` | the MIDI manufacturer id (one byte as two hexadecimal digits, or three bytes `00 xx xx` as six), when one exists; absent otherwise |
| `url` | the web address of the provider |
| `credit` | on a seeded entry, where the identifier, the name and the address were read, and the licence read there (section 4) |

The three kinds say who an entry is: a `maker` made the device or its software; a `community-firmware` is an alternative firmware made by people other than the maker; a `partner` is a project whose data formats or catalogues OSD carries. An entry never says how far to trust it (section 6). The list of kinds is closed; a new kind is a proposal.

An entry without `midi_manufacturer_id` is admitted. A maker that has one carries it; two entries never carry the same id, except that a community firmware may carry the id of the maker whose device it runs on, which is the reason `kind` exists beside the id.

### 3 Where the table lives, its format, its registration

**Where.** A data file of the repository, `schemas/providers.json`, beside `schemas/extensions.md`, served at a permanent address of the same family as the schemas (`https://w3id.org/osd/providers`). It is not part of the text of any version of the format.

**Format.** JSON, validated by a schema of its own, `providers.schema.json`: an object with `version` (a date, `YYYY-MM-DD`, of the last change) and `providers`, the array of entries. JSON because readers already read JSON and a pull request on it is checked by a program.

**Policy of registration.** Anyone may ask for an addition by pull request. A request gives, for an entry: the identifier, the name, the kind, the web address, and the MIDI manufacturer id when it exists with the place where it was read; and, for a `community-firmware` or a `partner`, one line saying what it supplies. The maintainers of the table review each request, check the entry at its source (for an id, the MIDI Association's published table), and merge it when the request is complete, the identifier is free and well formed, and the name is the provider's own. A request is refused with its reason written in the pull request.

**An identifier is never reused.** An entry is never deleted and its `id` is never given to another provider; one that has ceased to exist, or that was entered by mistake, stays with `deprecated: true` and, when another entry replaces it, `replaced_by`. So a descriptor that cites an identifier years later still cites the same provider.

**Its versions and the versions of the format.** The table has its own version (a date), not the format's: an addition does not make a new version of the format. A descriptor of any version of the format that has `provider` cites the same table, and a reader states the version of the table it carries. This follows the SPDX License List (a list with its own version, a release per change, cited by identifier from a document, the specification naming the list and not its entries) and the IANA registries (a registry beside the protocol, with a policy of registration, entries never reassigned).

Departures from the models, each with its reason:

- **From SPDX**, which publishes releases on a schedule with a steering group's review: the table releases at each merge, there being one owner and no committee, and the date is the version.
- **From IANA**, which has several policies by registry (Expert Review, Specification Required): the table has one, review by the maintainers, since entries are facts about who exists and not allocations of a protocol's number space.
- **From `schemas/extensions.md`**, a Markdown list for extension terms, listed to become a core term: the table is JSON, since a program reads it and a schema checks it, and nothing in it is promoted to a term; the extension list's rule that a listed entry has a stable URL is kept (`https://w3id.org/osd/providers#<id>`).

### 4 How the MIDI manufacturer ids seed the table

The table starts with the three partners the converter names (`electra-one`, `pencilresearch`, `knobkraft`, section 5), and with no maker. A maker is seeded from the MIDI Association's manufacturer ID table, for the devices that OSD describes, when a descriptor needs it. The numeric id of a maker and its name are facts about the world, restated in the table's own entries; the table does not copy the MIDI Association's table, its text or its layout, and a MIDI norm is never copied. The table credits the MIDI Association as the source of the ids, with the licence it publishes, read at its source and cited with its address (https://midi.org/manufacturer-id-numbers) at the time of the seeding, in the `credit` of each seeded maker. The table has no header and no maker entry until a descriptor needs one: the form of a header, the credit of the MIDI Association for the whole table, is decided before the first maker entry. An entry is added when a descriptor needs it, not in bulk. The review of a request for addition (section 3) checks the entry at that source.

### 5 What the converter 0.6 → 0.7 writes

A source of 0.6 has no field `provider`. For each type, the converter writes the line below (a source that already carries the key is said after the table); `_sources` becomes `sources` as proposal 0008 decides.

| `type` in 0.6 | `type` in 0.7 | `provider` | `notes` |
|---|---|---|---|
| `hardware-verified`, `hardware-observed`, `software-verified`, `vendor-doc`, `vendor-osc-api`, `vendor-self-described`, `third-party-osc`, `community-re`, `contributed`, `test-fixture` | the same | none written: the converter does not guess who a source's author is | unchanged |
| `electra-preset` | `contributed` | `electra-one` | first line `type in 0.6: electra-preset`, then the existing text unchanged |
| `pencilresearch` | `contributed` | `pencilresearch` | first line `type in 0.6: pencilresearch`, then the existing text unchanged |
| `knobkraft` | `contributed` | `knobkraft` | first line `type in 0.6: knobkraft`, then the existing text unchanged |
| `midnam` | `contributed` | none written: MIDNAM is a format, not a supplier; the source's `author` stays what the source says | first line `type in 0.6: midnam`, then the existing text unchanged |

The four product-named types lose their name in `type`. Each keeps the old word in `notes`, and the three that name a partner also give its identifier in `provider`; `midnam` gives none, and the table has no entry for it. `contributed` is written for the four because the data is a third party's, supplied by a partner's format and not checked by OSD; it is the 0.6 term for that ("contributed by a person or an agent, unchecked"). The nature of the data (a preset, a map, an adaptation, a name document) stays in the free text of `notes`, which the table does not carry and no field holds. The converter writes an identifier whether or not its table has the entry; the three above are seeded.

**A 0.6 source that already carries `provider`.** A well-formed identifier is kept, and wins over the one the type would give. Any other value stays under `_provider` (when that key is taken, `_provider_2`, then the next number), and the identifier the type gives, when it gives one, takes its place. So nothing is lost and the result is valid, the rule 0.7 already applies to texts.

Cases of the suite, under `conformance/0.7/convert/`: `source-product-types-become-contributed`, `source-evidence-type-unchanged` and `source-provider-already-there`.

**What an import writes.** An import writes `provider` only when its caller gives it: the format read does not say who supplied the file (a MIDNAM document or a CSV may come from a maker). `osd import` takes the provider as an option. No provider is guessed.

### 6 Trust is out of scope

The table states who an entry is, never how far to trust it. The levels of trust in a source are SyntHub's: they depend on a catalogue's own checks and policy, change with its experience, and one tool's judgement does not belong in a format that every tool reads. A reader or a catalogue that wants a level keeps its own table keyed by `provider` and `type`. So the standard stays neutral, and `type` still says the kind of evidence, which is a fact of the source.

### 7 Examples

A source of 0.7 converted from `electra-preset`:

```json
{ "type": "contributed",
  "provider": "electra-one",
  "notes": "type in 0.6: electra-preset\na preset of the controller, converted" }
```

A source of type `vendor-doc`, from a maker's manual:

```json
{ "type": "vendor-doc",
  "provider": "acme-synths",
  "title": "MIDI implementation, v1.2" }
```

An entry of the table:

```json
{ "id": "electra-one", "name": "Electra One", "kind": "partner",
  "url": "https://electra.one" }
```

and, as it would be seeded when a descriptor needs it, a maker:

```json
{ "id": "acme-synths", "name": "Acme Synths", "kind": "maker",
  "midi_manufacturer_id": "7D", "url": "https://example.org",
  "credit": "id and name read from the MIDI Association's manufacturer ID table, with the licence it publishes" }
```

A source with `provider: "Electra One"` is refused as `BadValue` (not of the form). A source with `provider: "a-new-firmware"`, well formed and absent from the reader's table, is read, returned as it is, and warned about.

### 8 What it changes in the other proposals

**In proposal 0008.** The single type `partner-data` has been withdrawn from it (its decision 9), and the alternative of that decision, an extension term per partner, is also set aside. For the four source types, 0008 refers to this proposal: in its table of changes (section 2), its paragraph on source types, its case of conformance 6 and its tension "No reader privileged". 0008 keeps the other changes of form. The four types become `contributed`, three of them with a `provider`, as section 5 says; 0.7 section 6.6 loses the four rows and gains no row for a partner.

**In proposal 0009.** Its example no longer has a source of type `partner-data`: it already cites the table, the `type` being `contributed` and the partner named by `provider`.

**In the text of 0.6.** Section 6.6 keeps its ten terms of evidence in 0.7; its four rows named after a product (`electra-preset`, `pencilresearch`, `knobkraft`, `midnam`) leave the closed list and are written by the converter as section 5 says. No version published changes.

### 9 Every kind of descriptor keeps its credit

The field `provider` stands in a source, so every descriptor that has sources can say who supplies them. In 0.7 a maker profile, a sound, a fingerprint, a firmware and an assembly (the `made_on` of a sound included) carry `sources`, with the definition and the rules of a device's: the same terms of `type`, the same `provider`, the same refusals, the same warning of a provider the table does not hold. Their `_sources` is refused, as a device's. So the charter's credit (every descriptor names its sources and their licence) has one field and one meaning in every kind.

Before 0.7 their `_sources` was a free note. The converter therefore judges it entry by entry and refuses nothing for it: an entry that is a source of 0.7 once carried (section 5 and proposal 0008) goes to `sources`; an object without `type` is carried as `contributed`, `no type in 0.6` first in `notes`, so that its address and licence stay credited; every other entry, and a note that is no list, is kept under `_sources_2` (or the next free number), which a reader warns of as a source not carried as credit, so that it is never silent.

A source also says the maker profile it comes from, in the key `profile`, a string. A resolving reader writes it when it keeps the credit of the profiles a resolved device takes blocks from: a field with one meaning, and not a line in `notes`. The rules of that resolution are those of the text (23.42 and 23.43). A `profile` that is no string is refused, and the converter keeps such a value under `_profile`.

## What a reference does

Read at its source on 2026-11-24:

- **SPDX License List** (https://spdx.org/licenses/; the repository https://github.com/spdx/license-list-data). A list of identifiers, each short and stable, cited by identifier from a document; it grows by request on the repository (a documented process for adding a license) and carries its own version and release date. The specification names the list, not its entries. A tool that meets an identifier it does not know does not fail on it (SPDX has the `LicenseRef-` form for what is not on the list). OSD takes the identifier cited, the list with its own version, growth by pull request, and the tolerance of an unknown one.
- **MIDI Association manufacturer ID table** (https://midi.org/manufacturer-id-numbers). Gives each manufacturer a one-byte or three-byte SysEx id, assigned by the MIDI Association on request. OSD takes the id as a fact for makers and credits the source; it does not copy the table or its text.
- **IANA registries** (https://www.iana.org/protocols; the policies of registration in RFC 8126). A registry sits beside the protocol, each registry with a named policy of registration, entries assigned and not reassigned. OSD takes the registry beside the format, the written policy and the entries kept for good.

Inside OSD, `schemas/extensions.md` registers extension terms, with a one-sentence definition and a stable URL each; the table follows its stable URL and departs from it for the reasons of section 3.

## Objections

None yet.
