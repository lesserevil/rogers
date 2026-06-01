# Backport Plan

**Status:** Draft  
**Plan:** plans/backport-plan.md  
**Depends on:** plans/architecture-plan.md, plans/release-management-plan.md  

---

## Summary

When a fix lands on `main` or a release branch, Rodgers determines whether that fix needs to be propagated to older active release branches (maintenance releases). It creates tasks to track the cherry-pick work, waits for human approval to proceed, and closes the loop when the backport is landed.

---

## What Needs Backporting

Rodgers evaluates every merged commit for backport candidacy.

### Automatic Backport Candidates

Any commit that is:
1. A bug fix (commit message or linked issue labeled `bug`)
2. A security fix, identified by:
   - The linked issue has a GH Security Advisory (GHSA) cross-referenced in its description
   - OR the commit message or linked issue contains a CVE reference (pattern: `CVE-YYYY-NNNNN`)
   - OR the linked issue is labeled `security`
   If any of these conditions are met, Rodgers treats the fix as security-relevant and assigns `priority=1` to the backport task.
3. A documentation fix that corrects harmful or dangerously outdated information

...must be evaluated for backport to all **active release branches**.

### Active Release Branches

A release branch is active if:
1. It has been released (tag exists)
2. The project has not announced end-of-life for that minor version
3. A human has not explicitly marked the branch as closed for new releases

Active branches are enumerated in `config.release.active_branches`. Rodgers reads this at startup.

### Non-Backport Candidates

The following are **not** backported automatically:
- Feature commits (new functionality, not bug fixes)
- Refactors with no behavioral change
- Test-only additions
- Documentation additions that are purely additive (new content, not corrections)

A human can override and request backport for any non-candidate by applying a `backport-me` label to the original GitHub issue.

---

## Backport Detection

On every triage run, Rodgers compares the set of merged commits on each active release branch against what has already been backported (tracked via tasks). For each unreported commit that meets backport criteria:

1. Identify the target release branches
2. Create one `backport` task per target branch
3. Post a comment on the original GitHub issue noting the backport is pending

---

## Backport Task

Each backport task is filed as follows:

```bash
backlog create \
  --title="Backport #{commit_sha_short} to {branch_name}" \
  --description="$(cat <<'EOF'
Plan: plans/backport-plan.md

Backport for: #{commit_sha} - {one-line commit message}
Source issue: #{number}
Target branch: {branch_name}

WHAT TO DO
Cherry-pick commit #{full_sha} to release/{X.Y}. Create a PR targeting
release/{X.Y} with the cherry-pick. Resolve any merge conflicts.

ACCEPTANCE
- [ ] Cherry-pick of #{sha} applies cleanly to release/{X.Y} (or conflicts resolved)
- [ ] PR is open targeting release/{X.Y}
- [ ] CI passes on the backport PR
- [ ] PR is merged or given explicit approval to close without merging

PITFALLS
- If the fix requires changes to shared library code that has diverged
  between main and the target branch, the cherry-pick may require
  manual conflict resolution. Document any non-trivial conflicts
  in the task before closing.
EOF
)"
  --type=chore
  --tag=rodgers:type=backport
  --acceptance="Backport #{sha} to {branch_name} is merged or explicitly closed without merging"
  --priority={1 for security, 2 otherwise}
```

The task links back to the original GitHub issue and the source commit via `discovered-from` if supported.

---

## Approval to Backport

Like releases, backports require human approval before Rodgers creates the PR. Rodgers requests approval via a GitHub Discussion in the same category used for releases (`release.approval_discussion_category`). The approval uses the same voting window and stale-threshold timing as release approvals (`release.voting_window_days`, `release.stale_threshold_days`).

**Vote tiebreaking: Most recent vote wins always.** A 👎 always halts execution regardless of when it arrives — even mid-flight. The most recent reaction is the absolute final answer.

```
## Backport Proposal

**Commit:** {sha} — "{message}"
**Source issue:** #{number}
**Target branch:** release/{X.Y}

This fix meets backport criteria. Approve by reacting 👍.
Backport will be filed as a PR targeting release/{X.Y}.
```

---

## Backport Execution

When approved, Rodgers:

1. Creates a branch `backport/{sha_short}/{branch_name}` from the target release branch head
2. Files a `chore` task (`rodgers:type=backport`) describing what needs to be cherry-picked, which release branch, and what the child task acceptance criteria are
3. Posts a comment on the original issue noting the backport is in progress and linking to the task

Rodgers does not perform the cherry-pick. The cherry-pick is work for an actor outside Rodgers, tracked via the `chore` task.

---

## Conflict Handling

If a cherry-pick has conflicts, Rodgers:

1. Files a `chore` task (`rodgers:type=backport-conflict`) noting the target branch, the source commit, and that merge conflict resolution is needed
2. Posts a comment on the original issue: "Backport needs to be applied to `release/{X.Y}` but there are merge conflicts. A human must resolve them. Task filed for tracking."
3. Closes the approval Discussion

Rodgers does not attempt the cherry-pick or any partial application. It files the task and moves on.

---

## Integration with Release Management

When a backport PR is merged to a release branch, Rodgers detects the merge and:
1. Updates the backport task status to closed
2. Checks whether this backport completes the set of needed backports for the version
3. If all critical backports are merged, files a task suggesting a patch release (see plans/release-management-plan.md)

---

## Configuration

```yaml
release:
  active_branches:
    - release/1.x
    - release/2.x
  # main is always implicitly included as a source
```

---

## Edge Cases

**Fix is already in the release branch.** Rodgers uses semantic equivalence to determine if the fix is already present — not just a textual SHA match. Before filing a backport task, Rodgers:
1. Compares the source commit's diff to the target branch's git history (textual match first)
2. If no exact textual match is found, Rodgers uses its LLM to judge whether the source commit and the target branch have functionally equivalent code — same behavior, even if implementation details differ
3. If semantically equivalent (LLM confirms behavior match), Rodgers marks the backport as not-needed, closes the task with a note explaining the finding, and posts a comment on the original GitHub issue noting that equivalent fix is already present
4. If ambiguous, Rodgers files the backport task anyway and notes the ambiguity in the task description, asking the human to confirm

**Backport PR would be empty (file not present in target branch).** Rodgers identifies this case before creating the PR and instead creates a `note` task: "Cannot backport #{sha} to {branch}: target file does not exist. Needs alternative approach."

**Human explicitly closes a backport.** Rodgers respects the closure. It does not recreate it.

**Security patch.** Rodgers detects a security patch when any of the following signals are present:

1. **GH Advisory match.** The merged commit or linked GitHub issue is connected to an open GitHub Security Advisory (GHSA) in the repository. Rodgers queries `repository.advisories()` to check.
2. **`security` label.** The GitHub issue has the `security` label (or the project's configured security label from `rogation.security_label`).
3. **CVE pattern.** The commit message or issue body contains a CVE identifier matching `CVE-\d{4}-\d{4,}` (e.g., `CVE-2024-12345`).

If any signal is present, Rodgers sets the backport task priority to `1` (highest) and may post a security advisory notification on the original issue directing users to the patched release — opt-in, not automatic.

---

## Acceptance Criteria

- [ ] CRIT-1: When a bug fix, security patch, or `backport-me` labeled issue is merged to main, Rodgers identifies all active release branches within one triage run
- [ ] CRIT-2: Rodgers files a `backport` task for each target branch within one triage run of detecting the candidate
- [ ] CRIT-3: Rodgers creates a GitHub Discussion for each backport and waits for human approval before opening a PR
- [ ] CRIT-4: A human 👍 approval triggers the creation of a backport branch and PR targeting the correct release branch within one triage run
- [ ] CRIT-5: If a backport has merge conflicts, Rodgers files a conflict-resolution task and posts an alert comment, but does not attempt autonomous conflict resolution
- [ ] CRIT-6: When a backport PR is merged, Rodgers closes the corresponding backport task and checks for release completeness
- [ ] CRIT-7: The backport approval Discussion body contains at minimum: the commit SHA, the commit message, the source GitHub issue number, and the target release branch — all extracted directly from the merged commit and linked issue at creation time
- [ ] CRIT-8: A 👎 reaction (or a rejection comment) on the approval Discussion halts the backport and Rodgers posts a comment acknowledging the rejection and asking for guidance within one triage run of detecting the reaction
- [ ] CRIT-9: If no 👍 or 👎 reaction is received within `release.voting_window_days`, Rodgers posts a reminder comment on the Discussion
- [ ] CRIT-10: If no human response is received within `release.stale_threshold_days` (total, including the voting window and any pings), Rodgers closes the Discussion, files a revisit task, and does not proceed with the backport
- [ ] CRIT-11: For backport approvals: any 👎 before the backport PR is created halts the backport; once the PR is created the vote is locked and subsequent 👎 is acknowledged but does not stop the work; conflicting simultaneous votes resolve to 👎 (halt + ask for clarification); votes on a stale-closed Discussion are ignored
- [ ] CRIT-12: Rodgers detects a security patch when any of: GH Advisory match (via `repository.advisories()`), `security` label on the issue (or `rogation.security_label`), or CVE pattern (`CVE-dddddddd`) in commit message or issue body; detected security patches are filed as `priority=1` tasks