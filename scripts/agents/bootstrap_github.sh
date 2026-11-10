#!/usr/bin/env bash
# Idempotent GitHub setup for the multi-agent workflow:
# labels, roadmap milestones, and the owner project board ("Status" mirror + "Sprint" field).
# Output: .github/agents-project.env (PROJECT_OWNER, PROJECT_NUMBER)
# Requires: gh auth with scopes repo + project (gh auth refresh -s project)
set -euo pipefail
cd "$(dirname "$0")/../.."

owner=$(gh repo view --json owner --jq .owner.login)

label() { gh label create "$1" --color "$2" --description "$3" --force >/dev/null; }
label "task"               "1D76DB" "Agent-executable task with a contract"
label "blocker"            "B60205" "Blocks one or more tasks"
label "needs-human"        "000000" "Requires an owner decision or credentials"
label "stage:backlog"      "EDEDED" "Not yet contracted"
label "stage:ready"        "C2E0C6" "Contract complete, can be picked"
label "stage:in-progress"  "0E8A16" "Worker assigned"
label "stage:in-review"    "FBCA04" "PR open, lead reviewing"
label "stage:blocked"      "B60205" "Waiting on a blocker issue"
label "kind:fix"           "D93F0B" "Bug or audit fix"
label "kind:refactor"      "5319E7" "Structure change, no behavior change"
label "kind:feature"       "0075CA" "New spec behavior"
label "kind:qa"            "BFD4F2" "Test infrastructure or coverage"
label "size:s"             "F9F9F9" "Under ~150 changed lines"
label "size:m"             "F9F9F9" "Under ~400 changed lines"
label "complexity:high"    "D4C5F9" "Worker runs on the strongest model"
for area in authorization rooms synchronization identity events infrastructure e2ee media push federation; do
  label "area:$area" "C5DEF5" "Bounded context: $area"
done

# Milestones mirror docs/process/ROADMAP.md (titles must match exactly).
milestone() {
  gh api "repos/{owner}/{repo}/milestones?state=all&per_page=100" --jq '.[].title' | grep -qxF "$1" \
    || gh api "repos/{owner}/{repo}/milestones" -f title="$1" -f description="$2" >/dev/null
}
milestone "M0 Stabilize"            "Close audit issues V1-V10; green QA gate on develop"
milestone "M1 Foundations"          "Async DB boundary, error model, test harness, CI, typed IDs"
milestone "M2 Accounts & Auth"      "Registration, login flows, UIA, devices, account management, profile"
milestone "M3 Rooms & Events"       "Full room lifecycle, state resolution inputs, redaction, relations, context"
milestone "M4 Sync"                 "Spec-complete /sync, filters, long-poll wakeups, presence, typing, receipts"
milestone "M5 E2EE"                 "Device keys, one-time keys, key claim/query, to-device, key backup, cross-signing"
milestone "M6 Media"                "Authenticated media v1, upload/download/thumbnail, config"
milestone "M7 Push & Notifications" "Pushers, push rules, notifications endpoint"
milestone "M8 Spec Completion"      "All remaining non-deprecated CS API v1.18 operations at 100% coverage"
milestone "M9 Compliance & Perf"    "Complement CS suite, load tests, release hardening"

# Project board: reuse the owner's board (title "Gridstack" or PROJECT_NUMBER in env file).
# Stage is mirrored into the built-in "Status" field; a "Sprint" number field is added if missing.
if [ ! -f .github/agents-project.env ]; then
  number=$(gh project list --owner "$owner" --format json --jq '.projects[] | select(.title=="Gridstack" or .title=="Gridstack Homeserver") | .number' | head -1)
  if [ -z "$number" ]; then
    number=$(gh project create --owner "$owner" --title "Gridstack" --format json --jq .number)
    gh project link "$number" --owner "$owner" --repo "$(gh repo view --json name --jq .name)" >/dev/null
  fi
  printf 'PROJECT_OWNER=%s\nPROJECT_NUMBER=%s\n' "$owner" "$number" >.github/agents-project.env
fi
source .github/agents-project.env
fields=$(gh project field-list "$PROJECT_NUMBER" --owner "$PROJECT_OWNER" --format json --jq '.fields[].name')
grep -qx Status <<<"$fields" || { echo "project $PROJECT_NUMBER has no Status field" >&2; exit 1; }
grep -qx Sprint <<<"$fields" || gh project field-create "$PROJECT_NUMBER" --owner "$PROJECT_OWNER" --name Sprint --data-type NUMBER >/dev/null
cat .github/agents-project.env
echo "bootstrap complete"
