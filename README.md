# bootstrap

A project template for new repos. Clone it (or copy/paste from it)
as a starting point and adapt to your project's specifics.

## This is a template, not a project

Don't try to "run" bootstrap. The quality-gate targets in the
Makefile (`fmt`, `build`, `test`, `lint`, `clean`) are deliberately
stubbed; there's no application code. Bootstrap ships an opinionated
baseline (`AGENTS.md`, `Makefile`, `docs/`, `plans/`, githooks) and
you adapt it to your project.

## Workflow

1. **Clone or fork** bootstrap into a new directory, or copy the
   files into an existing repo.

2. **Run `make init`.** Idempotent — safe to re-run.
   - `git init` (if the directory isn't already a git repo)
   - checks that `bd` (beads) is installed and on `PATH` — errors
     out with an install pointer if not
   - `bd init` (creating `.beads/`)
   - points `bd`'s sync remote at git's `origin`, if `origin` is set

3. **Ask an agent to customize the repo.** Point your agent at
   `AGENTS.md` § *Customize per project* — that section enumerates
   the per-project decisions to make: pick a license, fill in the
   real toolchain commands, wire up the pre-commit hook checks, list
   canonical user-doc filenames, document the configuration surface,
   and decide on the opt-ins (commit attribution, plan-ref linting,
   `CONTRIBUTING.md`, etc.).

   A reasonable first prompt to the agent after `make init`:

   > Read `AGENTS.md` and walk me through the *Customize per project*
   > checklist. Ask me one decision at a time so I can update the
   > right files. When the checklist is done, replace this `README.md`
   > with a project-specific one describing the project we just set
   > up — nothing from the template README should remain.

## Layout

- `AGENTS.md` — operating rules (read this first)
- `Makefile` — `make init` plus per-project quality-gate stubs
- `docs/` — user-facing documentation
- `plans/` — design docs and architecture notes
- `scripts/githooks/` — pre-commit hook scaffolding

## Replace this README

Once you've customized the repo, **replace this file** with a real
README describing *your* project — what it does, how to install and
run it, its constraints. Nothing from this template README belongs
in your downstream README.
