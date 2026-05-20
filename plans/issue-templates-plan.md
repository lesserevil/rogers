# Issue Templates Plan

**Status:** Draft  
**Plan:** plans/issue-templates-plan.md  
**Depends on:** plans/architecture-plan.md, plans/feature-bug-plan.md, plans/question-routing-plan.md, plans/triage-workflow-plan.md  

---

## Summary

Rodgers uses GitHub issue templates to drive structured community input. When a project has its own templates, Rodgers uses them. When a project has none, Rodgers supplies suggested default templates and helps requestors reformat freeform submissions to conform to a template when they submit without one.

The template fields map directly to the completeness requirements in plans/feature-bug-plan.md and plans/question-routing-plan.md — this is not accidental. Templates exist so the completeness check is satisfied at filing time, not discovered after the fact.

---

## Template Discovery

On first run (or when configured for a new repo), Rodgers checks for the presence of `.github/ISSUE_TEMPLATE/` in the target repository.

```
.github/ISSUE_TEMPLATE/
├── bug_report.md      → Rodgers uses this for bug reports
├── feature_request.md → Rodgers uses this for feature requests
└── question.md        → Rodgers uses this for questions
```

If all three are present: Rodgers adopts them as the project's canonical templates.

If some or all are missing: Rodgers generates suggested templates (see Default Templates below) and files a bead: "Project is missing issue templates — suggested templates available, review and commit."

Rodgers does not auto-commit suggested templates to the repo. A human reviews and commits them first. This is intentional — template choices are a project governance decision.

---

## Default Templates

When the project has no templates, Rodgers suggests these as defaults.

### Bug Report Template

```markdown
---
name: Bug Report
about: Report something that isn't working as expected
labels: bug
---

## Bug Summary
<!-- One-line description of the bug -->

## Environment
- OS: [e.g. Ubuntu 22.04, Windows 11, macOS 14]
- Version: [software version if known]
- Other relevant context: [GPU model, driver version, etc.]

## Steps to Reproduce
<!-- Numbered list of steps. Be specific. -->
1.
2.
3.

## Expected Behavior
<!-- What you expected to happen instead -->

## Actual Behavior
<!-- What actually happened -->

## Relevant Logs / Error Messages
<!-- Paste or describe any error output. Leave blank if none. -->

## Possible Cause
<!-- Optional: your theory on why this is happening. Leave blank if unknown. -->
```

### Feature Request Template

```markdown
---
name: Feature Request
about: Suggest a new capability or behavioral change
labels: feature
---

## Feature Summary
<!-- One-line description of the requested feature -->

## Use Case
<!-- Why do you need this? What problem does it solve? -->

## Proposed Behavior
<!-- How should this feature work once implemented? Be specific. -->

## Acceptance Criteria
<!-- Numbered list of conditions that prove the feature is correctly implemented. -->
<!-- Each criterion must be testable — "it works well" is not a criterion. -->
1.
2.
3.

## Alternatives Considered
<!-- Optional: other approaches you considered and why they don't work -->
```

### Question Template

```markdown
---
name: Question
about: Ask about how to use or configure the project
labels: question
---

## Question
<!-- State your question clearly. Be specific about what you've tried and what you're trying to achieve. -->

## Context
<!-- Provide enough context for someone to answer without来回往返. Include: -->
<!-- - What you were trying to do -->
<!-- - What you already tried -->
<!-- - Relevant version / configuration -->
```

---

## Template Conformance

### Non-Conforming Issues

An issue is **non-conforming** when it is filed without using any of the project's issue templates.

Rodgers detects this by checking for a special marker in the template (e.g., a `<!--` comment that won't appear in a freeform submission). If Rodgers finds the marker absent from a new issue, the issue was filed without a template.

### Rodgers Offer to Reformat

When Rodgers detects a non-conforming issue, it does not close or reject it. Instead, Rodgers posts a comment:

```
Hi @[requestor], thanks for reaching out! We use issue templates to make sure we gather all the information needed to understand and address your request.

It looks like this was submitted without a template. Would you like help reformatting it? I'll rewrite it using the [bug report / feature request / question] template based on what you've shared — just confirm below and I'll post the reformatted version for your review.
```

If the requestor replies affirmatively, Rodgers:
1. Reads the existing issue content
2. Maps it onto the appropriate template fields
3. Posts the reformatted issue as a comment on the original issue
4. Asks: "Does this look right? If so, I'll update the issue to use this format."

If the requestor approves: Rodgers edits the issue body to match the template and removes the `needs-information` label if present.

If the requestor declines: Rodgers accepts the freeform submission and proceeds with triage, applying the `needs-information` workflow if details are missing.

**Key principle:** Rodgers never reformats without explicit consent. The requestor always approves the reformat before it is applied.

---

## Cross-Reference: Template Fields vs. Completeness Requirements

The template fields are not arbitrary — they map directly to Rodgers' completeness check in plans/feature-bug-plan.md.

### Bug Report Completeness Map

| Template Field | Completeness Requirement |
|---------------|-------------------------|
| `## Environment` | Required for bug completeness |
| `## Steps to Reproduce` | Required for bug completeness |
| `## Expected Behavior` | Required for bug completeness |
| `## Actual Behavior` | Required for bug completeness |

A bug filed with all four sections filled is completeness-complete. Rodgers applies `ready-for-review` without requesting additional information.

### Feature Request Completeness Map

| Template Field | Completeness Requirement |
|---------------|-------------------------|
| `## Use Case` | Required for feature completeness |
| `## Proposed Behavior` | Required for feature completeness |
| `## Acceptance Criteria` | Required for feature completeness |

A feature filed with all three sections filled is completeness-complete.

### Question Completeness Map

| Template Field | Completeness Requirement |
|---------------|-------------------------|
| `## Question` | Required to proceed with doc search |
| `## Context` | Required to avoid往返 |

A question filed with both sections filled has enough context for Rodgers to search docs accurately.

---

## Discovery and Adoption

Rodgers **discovers** templates on startup by listing `.github/ISSUE_TEMPLATE/`. It **adopts** them by reading their field structure and using those fields as the completeness check anchors.

If a project later adds or modifies a template, Rodgers detects the change on its next run and updates its completeness anchors accordingly.

A bead is filed whenever Rodgers detects a template change: type=`infra`, description: "Template(s) in .github/ISSUE_TEMPLATE/ changed — completeness anchors updated. Please review that the new template fields still cover all required information per plans/feature-bug-plan.md and plans/question-routing-plan.md."

---

## Edge Cases

**Requestor submits via email (GitHub Email Reply).** GitHub email replies create issues without any template context. Rodgers detects these as non-conforming and follows the reformat offer flow. Email-submitted issues are identifiable by the absence of template markers and a sender pattern indicating an email reply.

**Requestor closes the issue before Rodgers offers reformat.** If the requestor closes the issue before Rodgers runs, no action needed. Rodgers checks issue open state before posting comments.

**Partial template use.** If a requestor uses the feature template but leaves `Acceptance Criteria` blank, Rodgers applies `needs-information` requesting only the missing field — not a generic request for everything.

**Custom templates with non-standard field names.** Rodgers maps fields by semantic content (e.g., any field containing "environment" or "system" maps to Environment), not by exact name match. Unknown fields are ignored.

---

## Configuration

```yaml
templates:
  auto_suggest: true          # File a bead with suggested tempaltes if none found (default: true)
  require_use: false         # If true, non-conforming issues are auto-closed (not recommended)
  reformat_consent: true     # Always ask requestor before reformatting (default: true, do not change)
```

---

## Acceptance Criteria

- [ ] CRIT-1: On startup, Rodgers detects whether `.github/ISSUE_TEMPLATE/` contains bug_report.md, feature_request.md, and question.md
- [ ] CRIT-2: When a project has no templates and `auto_suggest: true`, Rodgers files a bead with suggested default templates within one triage run
- [ ] CRIT-3: A non-conforming issue (filed without template marker) receives a reformat offer comment within one triage run
- [ ] CRIT-4: Rodgers never reformats an issue without the requestor's explicit approval
- [ ] CRIT-5: When a requestor approves a reformat, Rodgers posts the reformatted content as a comment for requestor review before applying it
- [ ] CRIT-6: All default template fields map to a completeness requirement in plans/feature-bug-plan.md or plans/question-routing-plan.md
- [ ] CRIT-7: A bug report with all required template fields populated transitions to `ready-for-review` without requesting additional information