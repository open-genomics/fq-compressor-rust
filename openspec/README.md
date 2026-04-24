# OpenSpec for fqc

This repository uses **OpenSpec** for change planning and living requirements.

## Layout

```text
openspec/
├── changes/
│   └── <change-id>/
│       ├── proposal.md
│       ├── design.md
│       ├── tasks.md
│       └── specs/
└── specs/
    └── <capability>/spec.md
```

## How to work here

1. Read the relevant living spec in `openspec/specs/`.
2. If behavior or structure needs to change, update or add a change under `openspec/changes/`.
3. Implement the scoped tasks.
4. Keep docs, workflows, and repository metadata aligned with the spec.

For this repository, OpenSpec is also used to keep the project in a clean, finishable state: fewer stale docs, fewer redundant workflows, and tighter repository hygiene.
