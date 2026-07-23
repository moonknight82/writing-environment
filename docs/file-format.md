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

Search indexes, word counts, excerpts, recent-item lists, and synchronization state do not belong in Markdown files. They are derived data and may be rebuilt.

Removed sheets are copied into the application's data directory before the project copy is removed. Each Trash item retains its original relative path and removal time, allowing restoration without placing application metadata inside the synchronized library. Empty Trash permanently removes only validated Trash items belonging to the currently open project; it does not remove project Markdown files or revision History.
