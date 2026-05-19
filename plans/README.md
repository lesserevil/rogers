# Plans

Design and implementation documentation for this project.
Architecture notes, proposed-but-not-yet-shipped features, internal
mechanism inventories, experimental design records.

If you're trying to learn how this project **works inside** or how
it **might work in the future**, you're in the right place. If
you're trying to learn how to **use** it, see [`../docs/`](../docs/).

## Plan docs are living specifications

Plan docs are **not** "internal notes that can lag behind." For any
class of work that goes through the design-first pipeline, the
relevant plan doc is updated **before** or **alongside** the code
change. See [`../AGENTS.md`](../AGENTS.md) for the full
plans→beads→code→plan-complete workflow.

## Required shape for plan docs

Every plan needs an **Acceptance Criteria** section — testable,
enumerated items that define what "this plan is complete" means:

```markdown
## Acceptance Criteria

- [ ] CRIT-1: <testable claim, e.g. "Native client round-trips
      text/plain clipboard updates with echo suppression — covered
      by `loop_guard_swallows_just_applied` and the loopback
      integration test in `tests/e2e`.">
- [ ] CRIT-2: ...
```

Each item must be testable (a passing test, a CLI invocation with
expected output, a manual procedure with a clear pass/fail). Vague
criteria ("works well", "is robust") do not count.

Each bead opened against a plan must reference it in the
description (`Plan: plans/<name>.md §<section>`) and may mirror the
acceptance criterion with `--acceptance="..."`.

## Contents

_No plans yet — create `plans/<feature>-plan.md` when starting
design work._

### Not subject to the sync rule

- Top-level brainstorming notes / roadmap shapes that haven't
  crystallised into a plan yet.
- One-off validation reports (e.g. dated `*-validation-*.md` or
  `*-review-*.md` files) — point-in-time snapshots, not
  specifications.
