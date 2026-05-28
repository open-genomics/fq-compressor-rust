# devcontainer-interface

## Why

The repository's developer-environment policy has drifted inside the devcontainer contract itself:

- the pinned toolchain is split across the image, features, and bootstrap script
- bootstrap behavior is implied rather than stated for create/start entry points
- cache ownership is partial and not documented as part of the interface
- container lifecycle behavior is implied rather than stated

This change creates a bounded OpenSpec contract before any implementation work starts.

## What changes

- define the supported devcontainer environments for `fqc`
- pin the devcontainer toolchain and repo-local utilities as part of the interface
- specify lifecycle guarantees for create/start bootstrap phases
- specify which caches the container owns and which state must remain disposable
- keep the scope limited to the repository devcontainer contract and its supporting docs

## Non-goals

- changing CI, Pages, or Copilot workflow files in this slice
- redesigning CI, Pages, or release workflows
- introducing new container runtimes or extra environment classes
- widening the repository's public feature surface
