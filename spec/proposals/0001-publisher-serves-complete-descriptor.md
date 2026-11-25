# 0001 — Whoever publishes a catalogue should serve the complete descriptor

## Status

- State: `candidate`
- Candidate since: 2026-11-24
- Under the trial: —

This proposal decides nothing until it is accepted: the content of 0.7 is the owner's. It is a `candidate` since 2026-11-24, waiting at least 14 days, open to objections.

## What it changes

The text of 0.7: a recommendation (SHOULD) addressed to a publisher of a catalogue.

## Why

A client that looks for how a device speaks needs nothing of how the catalogue is built (`docs/ARCHITECTURE.md`, decision 4: « Inheritance is for authors; a client is served the complete descriptor »). A descriptor with links forces the client to resolve them.

## The change

A publisher of a catalogue SHOULD serve, for each device, the complete descriptor: the descriptor of the same version, without links, as `osd::complete_descriptor` derives it (`osd/docs/INTERFACE.md`).

## What a reference does

Three references were read at their source on 2026-11-24.

**JSON Schema bundling.** Address: JSON Schema 2020-12 core, https://json-schema.org/draft/2020-12/json-schema-core, §9.3.1 "Bundling". Fact read there: bundling takes the references (`$ref`) to an external resource and embeds the referenced resources in the referring document; each embedded resource identifies itself with its own `$id` (and SHOULD carry `$schema`), and the references in the containing document "MUST NOT be changed" (they now resolve to the embedded resource), so that the output is identical. The embedded resources sit under `$defs` at the root.

**npm.** Address: the registry document of one version, `https://registry.npmjs.org/express/4.18.2`. Fact read there: the registry serves, for a version, a JSON document with the name, the `license`, the `dependencies` as the author declared them (ranges such as `"etag": "~1.8.1"`, not resolved versions) and the `dist.tarball` address of the published archive. The registry does not inline the dependencies' documents: the client resolves them.

**Homebrew's JSON API.** Address: https://formulae.brew.sh/docs/api/ and `https://formulae.brew.sh/api/formula/wget.json`. Fact read there: the formula, written by its author in Ruby, is served as pre-computed JSON (`brew info --json`), one document per formula, a plain JSON object a client reads without Ruby. It carries `license` (`GPL-3.0-or-later` for wget), `homepage` and `urls` (the sources), and lists `dependencies` by name, without inlining them.

**Where 0001 follows them.**

- Like Homebrew, the publisher serves a pre-computed JSON document that a client reads without the author's machinery (Ruby there, links here), and the document keeps its credit: Homebrew keeps `license` and `urls`, 0001 keeps the sources and the licence of the descriptor.
- Like bundling, what a link pointed to is taken into the document, and the document keeps the identity of its format: the `$schema` of the descriptor is unchanged, as an embedded resource keeps its `$schema` and `$id`. The complete descriptor is a descriptor of the same version, so a reader of the first level reads it as it is.
- Like npm, the authored form stays available beside the served one (the repository and the tarball there, the descriptor with its links in the catalogue here).

**Where 0001 departs, and why.**

- npm and Homebrew do not resolve what the author declared: their documents list dependencies and the client resolves them. 0001 resolves the links, because decision 4 of the architecture puts that work on the publisher: a client of a catalogue looks for how a device speaks and nothing else. The reason is the client's, which the references do not share.
- Bundling resolves fully and keeps every reference valid and every URI unaltered, to give identical output. 0001 resolves **partially**. What is resolved: the blocks a maker profile brings and the MIDI that `midi_of` shares, merged by the rules of the version. What is lost: `midi_of` itself (the routes of its target stay), and the name of the profile in `identity` and `programs` (the keys it brought stay). The output is therefore not identical to the document with its links, and does not claim to be; it is the same device as its firmware runs it. The loss is acceptable because what a client needs, how the device speaks, is complete, and what the lost fields said (where a block came from) is not what the client looks for.
- What stays is the key of provenance and credit: the sources and the licence remain in the complete descriptor, as `license` and `urls` remain in Homebrew's document. Credit is a priority of the domain, and a resolution that dropped it would weaken it; this one drops the link and keeps the credit.

**Does a publisher also serve the descriptor with its links?** It MAY, as npm serves the archive beside the version document, but 0001 asks nothing of it: the recommendation is the complete descriptor. The two are told apart by the address a publisher gives them, and by the content: not by `$schema`, not by a field. Reason, following the references: bundling and Homebrew keep the identity of the format (`$schema`, document shape) and mark the resolved form with nothing new; a second `$schema` would make a second format to read, and a marker field would be a second meaning for a field. The complete descriptor is a descriptor with no link: a client that finds `midi_of` or a profile name in a document holds the one with its links. A publisher that serves both gives them distinct addresses of its own and says which is which in its documentation; the address scheme of a catalogue is the object of 0002, not of this text.

## Objections

The last question, whether the text of 0.7 should fix a way to tell the two descriptors apart (an address convention, a field) or leave it to the publisher, is answered (2026-11-24): the text of 0.7 fixes no way, and the publisher chooses. Reason: the three references read (JSON Schema bundling, npm, Homebrew) mark nothing in the document, and a field would say a second time what the absence of links already says. None remains open. The proposal is a `candidate` since 2026-11-24, waiting at least 14 days, open to objections.
