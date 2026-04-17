# Technical RFCs (Request for Comments)

This directory contains technical design documents and architecture decisions for fqc.

## Overview

RFCs define **how** the system implements product requirements. They capture design decisions, trade-offs, and technical specifications.

## Active RFCs

| RFC | Title | Status | Created |
|-----|-------|--------|---------|
| [0001](./0001-core-architecture.md) | Core Architecture | ✅ Accepted | 2024-01-15 |
| [0002](./0002-compression-algorithms.md) | Compression Algorithms | ✅ Accepted | 2024-01-15 |
| [0003](./0003-pipeline-architecture.md) | Pipeline Architecture | ✅ Accepted | 2024-01-20 |

## RFC Status Workflow

```
📋 Draft → 🔄 Proposed → ✅ Accepted → 🔨 Implemented → 📦 Final
                                    ↓
                                ❌ Rejected
                                    ↓
                                🗑️ Withdrawn
```

## RFC Naming Convention

```
NNNN-short-title.md

NNNN: 4-digit sequential number (0001, 0002, ...)
short-title: kebab-case title
```

## RFC Template

```markdown
# RFC-NNNN: Title

## Status
- Draft: YYYY-MM-DD
- Proposed: YYYY-MM-DD
- Accepted: YYYY-MM-DD

## Summary
Brief description of the proposal.

## Motivation
Why this design is needed.

## Design Details
Technical specification.

## Alternatives Considered
Other approaches evaluated.

## Unresolved Questions
Open issues to be resolved.

## References
Related documents and resources.
```

## Creating a New RFC

1. Copy the template above
2. Name it with the next sequential number
3. Fill in all sections
4. Submit for review
5. Update status as it progresses

## Related Directories

- [../product/](../product/) - Product requirements (what to implement)
- [../api/](../api/) - API definitions
