# Triage labels

These are the canonical label names used by the `triage` skill.

| Role | Label | Description |
|------|-------|-------------|
| `needs-triage` | `needs-triage` | Maintainer needs to evaluate |
| `needs-info` | `needs-info` | Waiting on reporter for more information |
| `ready-for-agent` | `ready-for-agent` | Fully specified, ready for AFK agent pickup |
| `ready-for-human` | `ready-for-human` | Needs human implementation |
| `wontfix` | `wontfix` | Will not be actioned |

## Creating labels

If these labels don't exist in the repo yet, create them:

```bash
gh label create "needs-triage" --description "Needs maintainer evaluation"
gh label create "needs-info" --description "Waiting on reporter for more information"
gh label create "ready-for-agent" --description "Fully specified, ready for AFK agent pickup"
gh label create "ready-for-human" --description "Needs human implementation"
gh label create "wontfix" --description "Will not be actioned"
```
