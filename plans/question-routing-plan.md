# Question Routing Plan

**Status:** Draft  
**Plan:** plans/question-routing-plan.md  
**Depends on:** plans/architecture-plan.md  

---

## Summary

When a community member files an issue labeled `question`, Rodgers determines whether the answer already exists in the project's documentation. If it does, Rodgers posts a comment with a link. If it does not, Rodgers treats it as a documentation gap and the workflow in this document applies.

---

## Workflow

### Step 1: Classify as Question

When the Triage Engine encounters an issue labeled `question`, it passes it to the Question Router. If the issue has no existing comments from Rodgers, proceed to Step 2.

### Step 2: Search Documentation

Rodgers searches `docs/` for content relevant to the question. Search is keyword-based over the full text of documentation files. The goal is recall: better to find a partial match than miss the right doc.

**Search scope:**
- `docs/**/*.md` — all user-facing documentation

**Search targets:**
- Filenames
- Section headings
- Paragraph text

### Step 3a: Documentation Found

Rodgers posts a comment on the issue:

```
Hi @[requestor], thanks for the question!

The answer to your question is covered in [docs/filename.md §section-title]().

[One-sentence summary of the relevant content, extracted from the doc.]

If this doesn't fully answer your question, please let us know and we will follow up.
```

Rodgers then closes the issue or leaves it open based on the answer quality — if the linked doc fully answers, close the issue; if it's partial, leave it open and wait for follow-up.

### Step 3b: Documentation Not Found

The absence of documentation is a documentation gap. Rodgers treats this as a `docs` work item and proceeds:

1. **File a `docs` bead** with:
   - Type: `docs`
   - Title: `Answer question: [one-line restatement of the question]`
   - Description: the full question text from the issue, the full issue body, and any relevant context
   - Acceptance: a new section in the relevant doc that answers the question; the section must be linked from the issue when filed
   - `discovered-from` link to the originating issue

2. **Post a comment** on the GitHub issue:

```
Hi @[requestor], thanks for the question! We do not currently have documentation that answers this. We have opened a task to add an answer to our documentation — it will be linked here when complete.
```

3. **Label the issue** with `needs-documentation`. Remove `question` label.

### Step 4: Update Docs (Sideband)

The human or agent working the `docs` bead updates the relevant documentation file with a section that answers the question. When the doc section is written and checked in, the implementer posts the link as a comment on the GitHub issue and closes the issue.

### Step 5: Sync Bead to GitHub Issue

When the `docs` bead is closed, Rodgers verifies that the GitHub issue has a comment linking to the new documentation. If the comment is missing, Rodgers posts the link.

---

## Question Router Decision Tree

```
issue labeled question?
  │
  └─► Rodgers has already commented?
        │
        └─► YES ──► No-op (already handled)
        │
        └─► NO ──► Search docs/
              │
              ├─► Doc found ──► Post comment with link, close issue
              │
              └─► Doc not found ──► File docs bead, comment on issue,
                                    label needs-documentation, remove question
```

---

## Edge Cases

**Question is not a question.** If Rodgers determines the issue is actually a bug report or feature request in disguise, it re-labels it accordingly and hands it to the Feature/Bug workflow (plans/feature-bug-plan.md) instead.

**Question is too vague to answer.** Rodgers posts a comment asking for clarification before the doc search. Once clarification is received, it restarts from Step 2.

**Multiple questions in one issue.** Treat as one question — the primary question. If the issue clearly contains semantically distinct questions, file separate `docs` beads for each.

**Doc search returns false positives.** Rodgers presents the most relevant doc link. If the requestor says the linked doc doesn't answer their question, treat as Step 3b — file a `docs` bead.

**Requestor adds more context after Rodgers responds.** Rodgers processes the new comment as a new triage event — restarts from Step 1.

---

## Acceptance Criteria

- [ ] CRIT-1: When a `question` issue exists and docs exist that answer it, Rodgers posts a comment within one triage run with the correct doc link
- [ ] CRIT-2: When a `question` issue exists and no docs answer it, Rodgers files a `docs` bead within one triage run and posts an acknowledgment comment on the issue
- [ ] CRIT-3: When the `docs` bead is closed, Rodgers verifies the GitHub issue has a documentation link and closes or updates the issue accordingly
- [ ] CRIT-4: Rodgers never closes a question issue without either answering it or filing a docs bead
- [ ] CRIT-5: Rodgers never routes to a non-question issue through this workflow