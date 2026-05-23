# Error Enum Audit

This document maps each variant of the monolithic `RogersError` enum to its originating module. This will guide the creation of per-module error types.

| Variant | Originating Module | Underlying Error Type |
|---------|-------------------|-----------------------|
| Config | `config::error` | `ConfigError` |
| GitHub | `github::error` | `GitHubError` |
| Beads | `beads::error` | `BeadsError` |
| Release | `release::error` | `ReleaseError` |
| BackportExecution | `backport::execution` | `BackportExecutionError` |
| BackportConflict | `backport::execution` | `BackportConflictError` |
| FeatureBug | `feature_bug::error` | `FeatureBugError` |
| Init | `init::error` | `InitError` |
| Checks | `checks::error` | `ChecksError` |
| Triage | `triage::error` | `TriageError` |
| Doctor | `doctor::error` | `DoctorError` |
| QuestionRouter | `question_router::error` | `QuestionRouterError` |
| Llm | `llm::error` | `LlmError` |
| Auth | `github::auth` | `AuthError` |