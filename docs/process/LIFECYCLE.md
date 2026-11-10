# Development lifecycle (multi-agent)

How Gridstack is built without the owner in the loop. The owner opens a terminal, runs `/lead`, and reads sprint reports.

## Roles

| Role | Who | Owns | Never does |
| --- | --- | --- | --- |
| Owner | human | Roadmap priorities, `needs-human` issues, secrets, releases | Day-to-day task management |
| Lead | main Claude session running the `/lead` skill | Roadmap → milestones → sprints → task contracts; dispatch; review; merge; sprint reports; blocker triage | Writes feature code itself (except trivial review fix-ups < 10 lines) |
| Worker | `worker` subagent (`.claude/agents/worker.md`), one per task, in its own git worktree | Research → code → tests → QA gate → PR | Changes scope, edits other tasks' files, merges |

Subagents cannot spawn subagents, so the lead must be the top-level session.

## Artifacts and sources of truth

| Thing | Source of truth |
| --- | --- |
| Roadmap and milestone definitions | `docs/process/ROADMAP.md` + GitHub milestones (same titles) |
| Task contract | GitHub issue body (template `.github/ISSUE_TEMPLATE/task.md`) |
| Task stage | `stage:*` label (board "Status" field mirrors it; Blocked is label-only via `scripts/agents/task.sh`) |
| Sprint membership | Project "Sprint" number field (board #12 "Gridstack") + list in `docs/sprints/sprint-NN.md` |
| Project state | `generated/reports/state.md` (`scripts/agents/project_state.sh`) |
| Spec completeness | `generated/reports/spec-coverage.md` (`scripts/agents/spec_coverage.py`) |
| Quality gate | `generated/reports/qa-gate.txt` (`scripts/agents/qa_gate.sh`) |
| Test requirements | `docs/process/QA.md` |

## Task state machine

```
backlog ──contract written──▶ ready ──worker dispatched──▶ in-progress ──PR opened──▶ in-review ──approved+merged──▶ done
                                ▲                              │                         │
                                │                              ▼                         ▼
                                └──blocker closed──────── blocked              changes requested (≤3 rounds,
                                                                               then lead splits or re-contracts)
```

Rules:
- A task enters `ready` only if every template section is filled, it is sized `size:s` or `size:m`, and all `Depends on` issues are closed.
- At most **3 workers in parallel**, and parallel tasks must not touch the same files (lead checks the context packs). Migrations are serialized: only one in-flight task may add a migration.
- After 3 review rounds without approval, the lead stops the worker, records what was learned on the issue, and re-contracts it smaller.

## Sprint loop (one `/lead run` = one sprint)

1. **Sense.** `scripts/agents/project_state.sh`. Read `state.md` only. If the working tree is dirty on a non-`develop` branch, treat it as owner work in progress: do not discard it; finish it as a task on that branch or stash it under a named stash and note it in the sprint report.
2. **Bootstrap (once).** If `.github/agents-project.env` is missing, run `scripts/agents/bootstrap_github.sh`. If `generated/openapi/matrix-openapi.json` is missing, run task `Matrix: Sync & Generate OpenAPI`.
3. **Plan.** Pick the lowest open milestone. Select 4–8 `ready` tasks (contract new ones from `spec-coverage.md` missing list and open bugs if fewer). Assign `Sprint = N`. Write `docs/sprints/sprint-NN.md` with goal, task list, and the dependency order.
4. **Execute.** Dispatch workers (worktree isolation, background). Workers report `DONE <pr>` or `BLOCKED <issue>`.
5. **Review.** For each PR: QA gate result, diff vs. contract, review checklist below. Approve → squash-merge into `develop`, delete branch, `stage done`. Reject → precise numbered change list sent to the *same* worker (SendMessage, keeps its context).
6. **Triage blockers.** Actionable → convert to task (`ready`) in the current or next sprint. Needs owner → label `needs-human`, skip, continue with other work. Never wait on the owner.
7. **Close.** Rebase-free: `develop` must pass `qa_gate.sh` after the last merge. Regenerate coverage. Finalize sprint report (done, carried over, blockers, coverage delta, test count delta, next sprint goal). Commit the report to `develop`.
8. **Continue.** If invoked as `/lead auto`, start the next sprint; stop when the milestone list is exhausted, on a red `develop` gate that cannot be fixed in one task, or when only `needs-human` work remains.

## Review checklist (lead)

1. `qa-gate.txt` says `QA GATE PASSED` on the PR branch.
2. Every acceptance criterion maps to a test or code line. Missing → reject.
3. Layering per `src/services/AGENTS.md` (handlers/service/persistence). No DB in handlers, no HTTP in services.
4. No panics/`unwrap` on request paths; errors are typed and mapped to Matrix `errcode`.
5. Minimality: diff contains nothing outside the contract (no drive-by refactors, no new abstractions with one call site).
6. Performance: no blocking calls on async runtime, no N+1 queries, no full-table scans on hot paths, no new global locks.
7. Spec: response field names and status codes match `generated/openapi/matrix-openapi.json`.

## Branches and commits

- Base branch: `develop`. Task branch: `task/<issue>-<slug>`. One PR per task, `Closes #<issue>`.
- Squash merge. Commit title: `<kind>(<area>): <summary> (#<issue>)`.
- Lead never force-pushes `develop`. Workers never push to `develop`.

## Token budget rules

- Lead reads `state.md`, `spec-coverage.md`, issue titles, and diffs — not whole source files.
- Every task carries a **context pack** (exact files + line ranges). Workers read those first and `generated/reports/repo-map.md` instead of exploring.
- Workers use `rg`/`sed -n` ranges, never whole-file reads of files > 400 lines unless editing them.
- QA output is reduced to PASS/FAIL lines plus a 60-line failure tail.
- Default worker model: `sonnet`. Tasks labeled `complexity:high` run with model `opus`.

## Definition of Done (whole roadmap)

- `spec-coverage.md` reports 100% of non-deprecated CS API v1.18 operations.
- Every operation has router tests for success and each documented error.
- Complement CS-API test subset passes (milestone M9).
- `develop` QA gate is green; no open `bug` or `security` issues.
