# Documentation

User-facing documentation for this project. Setup guides,
troubleshooting, operator how-tos, public API references.

If you're trying to learn how to **use** this project, you're in
the right place. If you're trying to learn how it **works inside**
or how it **might work in the future**, see [`../plans/`](../plans/).

## Contents

<!-- PROJECT: list canonical user-doc files here as they're added.
     Common entries:
     - `installation.md` — toolchain, per-OS deps, build features
     - `getting-started.md` — quick walkthrough
     - `configuration.md` — env vars, config files
     - `troubleshooting.md` — common failures
     - per-binary pages for each CLI -->

_No docs yet — add files here as user-visible surfaces appear._

## Keeping docs in sync with code

Per [`../AGENTS.md`](../AGENTS.md), every commit that changes
user-visible behavior must update the relevant doc in the same
commit. The pre-commit hook warns when staged code touches a
feature area whose doc isn't also staged.
