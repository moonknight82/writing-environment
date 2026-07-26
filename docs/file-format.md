# Markdown library format

## Library layout

```text
My Novel/
├── library.json
├── Draft/
│   ├── 01-opening.md
│   └── 02-arrival.md
├── Research/
│   └── lighthouse-notes.md
└── Attachments/
    └── lighthouse-map.jpg
```

Folders provide a portable, human-readable default organization. `library.json` stores application-only ordering and view preferences but is not required to read the manuscript.

## Sheet format

```markdown
---
id: 2dc3e241-7c53-45d7-b2a5-a3ad2c63bd10
title: The Arrival
tags:
  - draft
  - chapter-one
created: 2026-07-18T14:20:00-03:00
modified: 2026-07-18T15:04:00-03:00
---

The rain arrived before anyone expected it.
```

The stable UUID distinguishes a renamed or moved sheet from an unrelated new file. Unknown front-matter properties must be preserved when the application saves a sheet.

## Derived data

Search indexes, word counts, excerpts, recent-item lists, and synchronization state do not belong in Markdown files. They are derived data and may be rebuilt. A project included in universal sync may contain `.writing-environment-project-id`, a one-line UUID used only to keep its remote folder stable across computers. It is optional project-level metadata: Markdown discovery and editing do not depend on it, and it is never treated as a sheet.

Removed sheets are copied into the application's data directory before the project or Inbox copy is removed. Each Trash item retains its original relative path and removal time without placing application metadata inside manuscript folders. The interface aggregates the separate origin partitions, and Empty Trash permanently removes only validated items from all origins or the explicitly selected origin; it does not remove current Markdown files or revision History.
