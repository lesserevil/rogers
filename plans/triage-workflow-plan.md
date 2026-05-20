# Triage Workflow Plan

**Status:** Draft  
**Plan:** plans/triage-workflow-plan.md  
**Depends on:** plans/architecture-plan.md, plans/feature-bug-plan.md, plans/question-routing-plan.md, plans/release-management-plan.md, plans/backport-plan.md  

---

## Summary

This plan defines the triage state machine: how Rodgers processes every new and updated GitHub issue, classifies it, and advances it through the workflow. It is the central document that coordinates the Feature, Bug, Question, and Release workflows defined in their respective plans.

---

## Triage Loop

On every schedule tick, Rodgers processes all issues that have changed since the last run. It reads the full issue state (labels, comments, body, assignee) and applies the state machine.

The state machine is applied **per issue per run**. Rodgers never assumes an issue is in a particular state from a previous run — it always re-evaluates.

---

## Top-Level Classification

Every issue starts in one of these top-level categories:

| Label | Type | Workflow |
|-------|------|----------|
| `bug` | Bug report | plans/feature-bug-plan.md |
| `feature` | Feature request | plans/feature-bug-plan.md |
| `question` | Question | plans/question-routing-plan.md |
| (no relevant label) | Unclassified | Step 1 of this plan |

Rodgers determines the initial label:
- If the issue author or body suggests a bug → apply `bug`
- If the issue author or body suggests a feature request → apply `feature`
- If the issue is a question (phrased as a question, asking for help rather than requesting a change) → apply `question`
- Otherwise → apply `question` as a default (people filing issues often have questions they don't know how to phrase)

Labels are applied only when Rodgers is the first to triage the issue. If another label is already present, Rodgers respects it and uses the existing classification.

---

## State Machine

```
                     ┌─────────────────────────────────────────────┐
                     │  NEW / UNCLASSIFIED                        │
                     │  Rodgers applies initial label              │
                     └──────────────────┬────────────────────────┘
                                        │
                    ┌───────────────────┼───────────────────┐
                    │ bug               │ feature           │ question
                    ▼                   ▼                   ▼
              ┌──────────┐       ┌──────────┐        ┌──────────┐
              │INCOMPLETE│       │INCOMPLETE│        │INCOMPLETE│
              │  (bug)   │       │ (feat)   │        │(question)│
              └────┬─────┘       └────┬─────┘        └────┬─────┘
                   │                  │                   │
          All required info?           │                   │
              │               All required info?          │
              │                  │                          │
              ├─ NO ─────────────┼── NO ────────────── NO ─┘
              │                  │                        │
              ▼                  ▼                         ▼
        ┌───────────┐      ┌───────────┐           ┌──────────┐
        │ NEEDS     │      │ NEEDS     │           │ SEARCH   │
        │ INFO      │      │ INFO      │           │ DOCS     │
        └───────────┘      └───────────┘           └──────────┘
```

```
                                          ┌─────────────────────────────┐
                                          │  NEEDS-INFORMATION          │
                                          │  Requested [field] from      │
                                          │  requestor                   │
                                          └─────────────┬───────────────┘
                                                        │ Requestor responds
                                                        │ (new comment)
                                                        ▼
                                          ┌─────────────────────────────┐
                                          │  (back to top — reprocess    │
                                          │   as if new comment read)    │
                                          └─────────────────────────────┘
```

```
                                          ┌─────────────────────────────┐
                                          │  READY-FOR-REVIEW            │
                                          │  Rodgers: has enough info    │
                                          │  Label: ready-for-review     │
                                          └─────────────┬───────────────┘
                                                        │ Human applies
                                                        │ will-not-do
                                                        │ OR ready-for-work
                                                        ▼
                              ┌───────────────────────────┴───────────────────────────┐
                              │                                                           │
                              ▼                                                           ▼
                    ┌─────────────────┐                                    ┌───────────────────┐
                    │   WILL-NOT-DO   │                                    │   READY-FOR-WORK  │
                    │                 │                                    │                   │
                    │ Rodgers: inform │                                    │ Rodgers: epic +   │
                    │ requestor       │                                    │ child beads       │
                    │ Rodgers: close  │                                    └───────────────────┘
                    └─────────────────┘
```

---

## State Descriptions

### NEW / UNCLASSIFIED

**Entry:** Issue exists with no Rodgers-applied label from a prior triage run.

**Action:** Rodgers reads the issue body and author context, applies the appropriate initial label (`bug`, `feature`, or `question`), and applies any `bot_labels` detection (see triage configuration). Moves to `INCOMPLETE` or proceeds directly to the appropriate workflow.

### INCOMPLETE

**Entry:** Issue is labeled `bug`, `feature`, or `question` but is missing required information for its type.

**Action:** Rodgers posts a comment requesting the specific missing information. Applies `needs-information` label. The specific field(s) requested are enumerated in the comment.

**Exit:** When the requestor responds, the next triage run processes the new comment as a state machine restart from `NEW`.

### NEEDS-INFORMATION

**Stub state** — the `needs-information` label and the awaiting state are represented by the issue being in this labeled state without a Rodgers comment having been posted.

When Rodgers sees `needs-information` applied, it checks:
- Has the requestor responded since the label was applied?
  - Yes → remove label, restart from top
  - No → has it been more than 14 days? If yes, post a gentle ping comment. If still no response after 4 more runs, close with a stale comment.

### READY-FOR-REVIEW

**Entry:** Rodgers has applied `ready-for-review` after determining the issue is complete.

**Action:** Rodgers waits. It does not move the issue without a human action.

**Exit events:**
- Human applies `will-not-do` → transition to `WILL-NOT-DO`
- Human applies `ready-for-work` → transition to `READY-FOR-WORK`

### WILL-NOT-DO

**Entry:** Human has applied `will-not-do`.

**Action:** Rodgers posts a comment on the issue:

```
Thank you for this [bug report / feature request]. After review, we have decided not to pursue this at this time.

[Optional brief reason from the reviewer.]
```

Rodgers closes the issue. No further action.

### READY-FOR-WORK

**Entry:** Human has applied `ready-for-work`.

**Action:** Rodgers triggers the appropriate workflow:

- `bug` → creates epic bead + child beads following plans/feature-bug-plan.md
- `feature` → creates epic bead + child beads following plans/feature-bug-plan.md
- `question` → should not reach this state (questions are resolved differently)

Rodgers posts a comment linking to the epic bead and applies `in-progress` or a project-status label if configured.

---

## Compound Issues

**Mixed issue (bug + question + feature in one):** Rodgers splits the issue. If possible, it files a separate issue for each distinct concern and closes the original with a comment explaining the split. Each new issue is triaged independently. This prevents a single `will-not-do` on a feature request from accidentally closing a legitimate bug report.

**Epic-scale issue:** If the issue describes work that clearly spans multiple epics (e.g., "redesign the entire authentication system"), Rodgers files an assessment bead rather than a standard epic. The assessment bead is for a human + agent to evaluate scope before any implementation begins.

---

## Edge Cases

**Issue modified after passing READY-FOR-REVIEW.** If the requestor substantially updates an issue after Rodgers has marked it `ready-for-review` (new information, revised scope, changed acceptance criteria), Rodgers removes the `ready-for-review` label, re-evaluates completeness, and restarts from `INCOMPLETE` if needed.

**Human marks ready-for-work but Rodgers hasn't filed the epic yet.** This cannot happen in normal flow — Rodgers files the epic in the same run that it detects `ready-for-work`. However, if the run fails mid-execution, the next run detects the orphan state and files the epic.

**Human applies both labels or contradictory state.** Rodgers processes in priority order: `will-not-do` > `ready-for-work`. If both are somehow applied simultaneously, Rodgers defaults to `will-not-do`.

**Unknown label.** Rodgers ignores unrecognized labels and attempts to process the issue based on the GitHub issue body content only.

---

## Cross-References

- Bug/Feature workflow: plans/feature-bug-plan.md
- Question workflow: plans/question-routing-plan.md
- Release management: plans/release-management-plan.md
- Backport: plans/backport-plan.md
- Triage empties to done: steps in plans/feature-bug-plan.md and plans/question-routing-plan.md

---

## Acceptance Criteria

- [ ] CRIT-1: Every new issue is processed within one triage run and assigned an initial label (`bug`, `feature`, or `question`)
- [ ] CRIT-2: An issue in `INCOMPLETE` state is never moved to `READY-FOR-REVIEW` until the required information for its type is present
- [ ] CRIT-3: Rodgers transitions `READY-FOR-REVIEW` → `WILL-NOT-DO` or `READY-FOR-WORK` only on human action (human applies the label)
- [ ] CRIT-4: When transitioning to `WILL-NOT-DO`, Rodgers posts a closure comment and closes the GitHub issue within one triage run
- [ ] CRIT-5: When transitioning to `READY-FOR-WORK`, Rodgers files the epic bead + child beads within one triage run and posts a comment linking to the epic
- [ ] CRIT-6: An issue with `needs-information` that has had no response for more than 14 days gets a gentle ping. After 28 days total with no response, the issue is closed with a stale notice.
- [ ] CRIT-7: Rodgers never makes a human gate decision (will-not-do, ready-for-work) on its own — it only observes and acts on the human's label