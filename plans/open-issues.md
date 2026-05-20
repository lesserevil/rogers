# Rodgers Plans — Open Issues (deep dive remaining)

## CRITICAL

1. ✅ `src/config.rs` — FIXED: Configuration Schema added to architecture-plan.md
2. ✅ `rogers-templates/issue-templates/` fictional path — FIXED: Embedded in binary + `rogation.template_dir` override
3. ✅ LLM tool signatures absent — FIXED: LLM Tool Registry with 14 tools in architecture-plan.md
4. ✅ No backport approval Discussion AC — FIXED: CRIT-7 through CRIT-10 added to backport-plan.md

5. ✅ **ISSUE 5 — Epic-scale assessment bead has no procedure** — FIXED: Full procedure added to triage-workflow-plan.md (epic detection at READY-FOR-WORK, LLM-driven breakdown, children deferred with human review gate, CRIT-9/CRIT-10 added)
6. ✅ **ISSUE 6 — IN_PROGRESS state has no exit criteria** — FIXED: Passive/next-poll mechanism made explicit in triage-workflow-plan.md; stalled recovery with one-time alert; CRIT-11 added
7. ✅ **ISSUE 7 — Step 5 sync verification is described but not defined** — FIXED: Full procedure added to question-routing-plan.md (timing, verification method, if-missing path, already-closed path, API error retry path); CRIT-4 updated

## HIGH

8. **`bot_labels` is referenced but never defined**
   - Referenced triage-workflow-plan.md line 99 as "any `bot_labels` detection"
   - Not defined anywhere in plan, not in schema, not enumerated

9. **Approver tiebreaking is missing**
   - release-plan and backport-plan both use 👍/👎 voting
   - No rule for when two humans vote differently or when vote flips after execution starts

10. **Security patch detection is undefined**
    - backport-plan.md uses "priority=1" for security patches
    - No procedure: consult GH Security Advisories? CVEs? Keyword patterns? `[SECURITY]` tag?

11. **`substantial update` is LLM-judgment-only**
    - triage-workflow-plan.md rolls back `ready-for-review` on substantial updates
    - No objective criteria for what "substantial" means

12. **Code search scope is arbitrarily incomplete**
    - question-routing-plan.md §Step 2 lists *.rs, *.py, *.js, *.ts, *.go
    - Missing: C#, Java, Kotlin, Swift, Ruby, PHP

13. **Child bead granularity has no definition**
    - feature-bug-plan.md §Bead Breakdown: "one bead per logical unit of work"
    - No examples, no thresholds for too-granular vs too-coarse

14. **`blocker` defined by label only**
    - release-management-plan.md checks `blocker` label only
    - Doesn't cover: priority labels, milestone-linked issues, human-marked blockers

15. **Doc-gap workflow has a missing party**
    - question-routing-plan.md §Step 4: "update docs (sideband)"
    - Who writes the doc? How does Rodgers detect completion? What if doc doesn't fully answer?

16. **Equivalent fix detection is undefined**
    - backport-plan.md §Edge Cases: checks if fix "already present"
    - No procedure for semantic equivalence vs textual match vs "functionally same"

## MEDIUM

17. **`rogation.*` keys in schema — incomplete enumeration**
    - architecture-plan.md lists keys but "Relevant config keys" section missing much of `rogation`

18. **LLM prompt strategy is scattered**
    - No centralized prompt library; prompts ad hoc per state transition

19. **`bd` is never defined as a dependency**
    - Rodgers runs bd create/list/close throughout but no install procedure, no version requirement

20. **init-plan.md AC-5 and AC-6 are missing**
    - Lines 11-12 list AC-1 through AC-4, then skip to AC-7

21. **feature-bug-plan.md has broken internal cross-reference**
    - Line 86 says "see Step 5 of the state machine (plans/triage-workflow-plan.md)"
    - triage-workflow-plan.md has no numbered Step 5

22. **Doc template URL is a stub**
    - question-routing-plan.md lines 53+: doc link template has literal `(url)` placeholder

23. **GitHub Actions detection doesn't cover non-`upload-artifact` workflows**
    - init-plan.md only checks for `upload-artifact` steps
    - Misses: `aws s3 cp`, `gh release upload`, `docker push`

24. **Negotiation labels not enumerated in one canonical list**
    - Scattered across multiple plans; no single canonical enumeration

25. **README.md has no architecture overview or extension guide**