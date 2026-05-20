# Feature Request and Bug Report Plan

**Status:** Draft  
**Plan:** plans/feature-bug-plan.md  
**Depends on:** plans/architecture-plan.md, plans/triage-workflow-plan.md  

---

## Summary

Feature requests and bug reports follow a shared workflow with two phases: a **completeness check** (is there enough information?) and a **readiness phase** (awaiting human go/no-go). Rodgers acts as an interrogator when information is missing, and an analyst when information is sufficient. All work spawned from accepted issues uses the epic/child bead pattern.

This plan documents the completeness check, acceptance criteria requirements, and the ready-for-work handoff. The underlying state machine is defined in plans/triage-workflow-plan.md.

---

## Issue Types Covered

- `bug` — Bug reports (error messages, reproduction steps, unexpected behavior)
- `feature` — Feature requests (new capability, behavioral change, integration request)

---

## Completeness Check

Before any work can begin, Rodgers verifies that the issue contains sufficient information. Rodgers performs this check on every triage run for any `bug` or `feature` issue that has not yet been marked `ready-for-review`.

### Bug Report Requirements

A bug report is complete when all of the following are present:

1. **Behavior observed** — A description of what happened that the reporter believes is wrong
2. **Behavior expected** — A description of what the reporter expected to happen instead
3. **Reproduction steps** — A clear set of steps to reproduce the issue (or `N/A` if the bug is a crash or data corruption that cannot be reliably reproduced, with an explanation of why)
4. **Environment** — OS, version of the software, relevant hardware or runtime context

If any of the above are missing, Rodgers posts a comment requesting the specific missing information and applies the label `needs-information`. It then waits for the requestor to respond.

### Feature Request Requirements

A feature request is complete when all of the following are present:

1. **Use case** — Why the requester needs this feature (the problem they are solving)
2. **Proposed behavior** — How the feature should work once implemented
3. **Acceptance criteria** — How the requester (or the team) would verify that the feature is correctly implemented. This is a testable, enumerated list — not "it should work well"

If the feature request is vague (e.g., "it would be nice if X could do Y"), Rodgers asks the requester to clarify the use case and proposed behavior before proceeding.

### Requirements Not Met

When `needs-information` has been applied and the requestor has not responded within the configured stale timeframe (default: 14 days), Rodgers applies the `needs-information` policy:

- If the requestor has not responded after 2 consecutive triage runs with `needs-information` applied, Rodgers asks once more with a gentler ping
- If still no response after 2 more runs, Rodgers closes the issue with a comment: "We haven't heard back on the information needed to move this forward. If you still want to pursue this, please reopen with the requested details."

---

## Generated Acceptance Criteria

When Rodgers determines an issue is complete (Step 1 of the state machine), it writes a draft acceptance criteria section in a comment on the issue:

```
## Rodgers Generated Acceptance Criteria

- [ ] AC-1: [testable claim]
- [ ] AC-2: [testable claim]
```

The acceptance criteria are Rodgers' proposed criteria, derived from the issue content. A human reviewer may accept, reject, or modify these criteria before marking `ready-for-work`.

---

## Readiness Phase

When Rodgers determines the issue is complete, it:

1. Applies `ready-for-review` label
2. Posts a comment summarizing the completeness check result
3. Waits for a human to remove `ready-for-review` and apply either `will-not-do` or `ready-for-work`

### Human Decision Gate

**`will-not-do` path:**
- Human applies `will-not-do` label and removes `ready-for-review`
- Rodgers posts a comment acknowledging the decision: "Thank you for the report. This will not be pursued at this time." and closes the issue
- Rodgers informs the requestor in a comment: see Step 5 of the state machine (plans/triage-workflow-plan.md)

**`ready-for-work` path:**
- Human applies `ready-for-work` label and removes `ready-for-review`
- Rodgers creates an epic bead for the work (see Bead Breakdown below)
- Rodgers posts a comment: "This has been accepted for implementation. Tracking progress in [epic bead title]."

---

## Bead Breakdown

When `ready-for-work` is applied, Rodgers creates beads as follows:

### Epic Bead
- Type: `epic` or the appropriate type matching the issue classification
- Title: matches the issue title
- Description: `Plan: plans/feature-bug-plan.md §Bead Breakdown. GitHub Issue: #<number>.`

The epic description links to the GitHub issue and includes the full acceptance criteria copied from the issue.

### Child Beads
Rodgers analyzes the issue and breaks it into **one bead per logical unit of work** (see AGENTS.md §Beads must stand alone — required completeness).

Guidelines for child bead scope:
- One section of the acceptance criteria per child bead, or one cohesive implementation concern
- Each child bead must be self-contained: a naive but competent junior developer could implement it without consulting other beads or the epic description
- Edge cases and constraints from the issue description belong in the relevant child bead, not the epic

### Labeling Conventions

Beads are typed and labeled consistently:
- `type=feature` for feature implementation beads
- `type=bug` for bug fix beads
- `priority` set based on the issue priority and complexity

---

## Cross-References

- State machine: plans/triage-workflow-plan.md
- Architecture: plans/architecture-plan.md
- Epic/child documentation: AGENTS.md §Beads must stand alone

---

## Acceptance Criteria

- [ ] CRIT-1: A `bug` or `feature` issue with all required information fields populated transitions to `ready-for-review` within one triage run
- [ ] CRIT-2: A `bug` or `feature` issue with any required information field missing applies `needs-information` and requests only the missing specific fields (not a generic request)
- [ ] CRIT-3: When a human applies `will-not-do`, Rodgers posts a closure comment and closes the issue within one triage run
- [ ] CRIT-4: When a human applies `ready-for-work`, Rodgers creates an epic bead followed by child beads within one triage run
- [ ] CRIT-5: Each child bead is self-contained: a developer reading only that bead can implement it without consulting other beads or the parent epic
- [ ] CRIT-6: All acceptance criteria from the GitHub issue are copied into the epic bead description
- [ ] CRIT-7: Rodgers never moves an issue to `ready-for-review` without the minimum required information for the issue type