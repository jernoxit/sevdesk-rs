---
paths:
  - "docs/**/*.md"
---

<!-- Condensed from CLAUDE.md section "Der Fahrplan trägt den Stand, die Protokolle den Verlauf" (with its subsections); CLAUDE.md remains the full version, both are kept in step. -->

# Docs

## The roadmap and the stage protocols are never committed in a public repository

`docs/fahrplan.md` and `docs/*-protokoll.md` are in `.gitignore` and live only locally, because they name findings, decisions and sister repositories.

```text
bad:  git add docs/fahrplan.md
good: .gitignore lists docs/fahrplan.md and docs/*-protokoll.md
```

## The roadmap carries the state, the protocols carry the history

`docs/fahrplan.md` says where to, in which order, how far; the history (stages, findings, measurements) goes into a protocol file per stage next to it, leaving one paragraph with a link and the state in the roadmap.

## What holds without its stage goes into `docs/architektur-notizen.md`

A rule or a rejected option with its reason belongs there (create the file with the first entry).

## A superseded decision is deleted, not put beside the new one

The rejected option with its reason moves to `docs/architektur-notizen.md` so nobody proposes it again. The same holds for doc comments in code: the present state stands there, no "used to be".

## The register is the only list of open work

An open item gets a number, one sentence and its domain in the roadmap's register, before it is noted anywhere else. A leftover table in a protocol is not a list.

## A heading is a name, and headings are the index

`grep -n '^#'` gives the outline in one call and a `Read` with `offset`/`limit` reads one section, so a heading is a name rather than the start of a sentence (a date in brackets may follow). Exactly one `#` as title, one `##` per area, `###` per subject, `####` only inside a protocol, no skipped level.

```text
bad:  ### How we decided to cut the transport      (sentence; and ## skipped above it)
good: ### Transport-Zuschnitt (2026-10-01)
```

## A `###` block over about 300 lines is a candidate for its own file

A block that long no longer reads as one section; move it out and leave a link.
