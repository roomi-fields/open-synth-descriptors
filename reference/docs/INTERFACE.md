# osd-reference — interface

**Langage** : cargo

`osd-reference` publishes one program, `osd-reference`, and the functions it is made of. It is part of the public repository, which builds its site with it; it is not released on the registries.

## Overview

| Word | Shape | Returns | Refuses |
|---|---|---|---|
| `build` | `osd-reference build <schemas folder> <output folder>` | one page `<kind>.md` per schema of the folder; exit 0 | a folder without schema, a file that is not a JSON schema, a reference no file of the folder answers (exit 1, naming the file and the pointer); a missing argument (exit 2) |
| `count` | `osd-reference count <schemas folder>` | one line per kind, `<kind>: <described> of <total>`; exit 0 | as `build` |
| `undescribed` | `osd-reference undescribed <schemas folder>` | the properties without description, one per line, `<kind> <pointer>`, sorted; exit 0 | as `build` |

## The functions

| Function | Returns |
|---|---|
| `osd_reference::properties(schema: &Value) -> Vec<Property>` | every property of a schema: its pointer, its type, its description when it has one |
| `osd_reference::page(kind: &str, schema: &Value) -> String` | the page of a kind |

## What a property is

A property is a key of an object `properties`, at any depth, `$defs` included. It is described when its own schema object holds a `description`; the description of the target of a `$ref` does not count.

## The guards

- A test builds the page of the device schema of 0.6 and counts 296 properties.
- A test holds the floor of described properties of each kind, recorded in one file; the floor is raised in the commit that adds descriptions.
- A test compares the committed list of the properties without description with what `undescribed` writes now.
