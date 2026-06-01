---
id: TASK-157
title: Migrate project tracking to Backlog.md
status: Done
assignee: []
created_date: '2026-06-01 16:50'
updated_date: '2026-06-01 17:03'
labels:
  - tooling
dependencies: []
priority: high
ordinal: 2000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Move Rogers from the legacy tracker to Backlog.md using the oompah migration script. Update Makefile/bootstrap scripts, docs, config schema, source modules, and generated task files to use the Backlog.md task store from https://github.com/lesserevil/backlog.md. Verify with make fmt-check, make build, make test, and make lint.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Backlog.md CLI is installed from https://github.com/lesserevil/backlog.md via make ensure-backlog.
- [x] #2 Legacy tracker storage is removed and migrated tasks live under backlog/.
- [x] #3 Code, docs, scripts, and Makefile use Backlog.md from lesserevil/backlog.md.
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Migrated Rogers from the legacy tracker to Backlog.md using the oompah migration script, installed Backlog.md from lesserevil/backlog.md via make ensure-backlog, removed legacy task storage, updated docs/scripts/Makefile/config/source modules, and verified fmt-check/build/test/lint.
<!-- SECTION:FINAL_SUMMARY:END -->
