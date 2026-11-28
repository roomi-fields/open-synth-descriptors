# 0024 — A licence is an SPDX expression

## Status

- State: `draft`
- Candidate since: —
- Under the trial: —

## What it changes

The text and the schemas of 0.7: every `license` of a descriptor (of a source, of a sound, of a firmware) is an SPDX licence expression, where 0.6 admits one id.

## Why

What this brings to the sharing of sounds on devices: a source writes its true licence, an exception or a combination included, so the credit that travels with a sound or a device loses nothing in a note.

The owner asked for it on 2026-11-28. The case is SyntHub's: for the M-VAVE SMK-37, the licence of a source is `GPL-3.0-only WITH GPL-3.0-linking-exception`, which 0.6 refuses; the id alone would say less than the source grants.

The owner also asked, on 2026-11-28, that a licence id may start in lowercase, as SPDX writes it: a source under `curl`, `zlib-acknowledgement` or `bzip2-1.0.6` writes its licence as it is.

## The change

Section 6.7 of [0.7](../0.7.md): a `license` is an SPDX licence expression, or alone `public-domain`, `NOASSERTION` or `NONE`.

```
expression  = and-term *( "OR" and-term )
and-term    = term *( "AND" term )
term        = simple [ "WITH" exception ] / "(" expression ")"
simple      = license-id / "DocumentRef-" idstring ":" "LicenseRef-" idstring
license-id  = idstring, in which "+" is also admitted after the first character
exception   = idstring
idstring    = 1*( letter / digit / "." / "-" )
```

Only the space (U+0020) separates, one or more. `WITH` takes a space on each side. `AND` and `OR` take on each side a space or a parenthesis (`(MIT)OR(ISC)`). A parenthesis may have spaces inside it. No space stands at either end. `AND`, `OR` and `WITH` are no id and no exception; `public-domain`, `NOASSERTION` and `NONE`, whatever their case, are no operand and no exception of an expression. `WITH` binds tighter than `AND`, and `AND` tighter than `OR`.

A reader checks this shape and never the SPDX registry. The schemas of 0.7 hold the same shape in a pattern, which cannot count parentheses: a reader alone refuses unbalanced ones.

A licence id has the shape of an SPDX idstring, whatever its first character (`curl`, `bzip2-1.0.6`, `0BSD`). As in SPDX, two spellings of an id that differ only in case name the same licence; a reader gives each back as it is written, and a descriptor writes an id as the SPDX list spells it, which no reader checks.

0.4, 0.5 and 0.6 judge a `license` as before. Every single id 0.6 admits is admitted by 0.7, the words `AND`, `OR` and `WITH` excepted, and the conversion keeps it. The table of 0.4 to 0.5 keeps the shape from before 0.7: an id that does not start with a capital, in a field it judges, stays `NotConvertible`.

## What a reference does

The SPDX specification 2.3, annex "SPDX License Expressions", is the form Cargo (the `license` field of a manifest) and npm (the `license` field of `package.json`) take. OSD takes its operators, its precedence and its parentheses. The annex was not read again at its source while this proposal was written.

Where OSD departs:

- operators are written in capitals only, and the special values never stand inside an expression (SPDX 3 differs); each special value has one spelling;
- the expression is a structure inside a string, which the frame of the specification refuses elsewhere (R8): the form Cargo and npm write is kept, and the schema cannot count its parentheses.

## Objections

Each objection raised during the candidate state, with its answer.
