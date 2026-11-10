#!/usr/bin/env bash
# Snapshot of project state for the lead agent and the human owner.
# Output: generated/reports/state.md
set -uo pipefail
cd "$(dirname "$0")/../.."
out="generated/reports/state.md"
mkdir -p "$(dirname "$out")"

python3 scripts/agents/spec_coverage.py >/dev/null 2>&1 || true

{
  echo "# Project state — $(date -u +%Y-%m-%dT%H:%MZ)"
  echo
  echo "## Git"
  echo "- branch: $(git branch --show-current)"
  echo "- head: $(git log -1 --format='%h %s')"
  dirty=$(git status --porcelain | wc -l)
  echo "- uncommitted paths: $dirty"
  [ "$dirty" -gt 0 ] && git status --porcelain | head -20 | sed 's/^/  - /'
  echo "- worktrees: $(git worktree list | wc -l)"
  echo
  echo "## Spec coverage"
  if [ -f generated/reports/spec-coverage.json ]; then
    jq -r '"- \(.implemented)/\(.total) operations (\(.percent)%)"' generated/reports/spec-coverage.json
  else
    echo "- unknown: generated/openapi/matrix-openapi.json missing"
  fi
  echo
  echo "## Last QA gate"
  if [ -f generated/reports/qa-gate.txt ]; then sed 's/^/- /' generated/reports/qa-gate.txt; else echo "- not run"; fi
  echo
  echo "## GitHub"
  echo "### Open milestones"
  gh api "repos/{owner}/{repo}/milestones?state=open&sort=due_on" \
    --jq '.[] | "- \(.title): \(.closed_issues) closed / \(.open_issues) open"' 2>/dev/null || echo "- unavailable"
  for label in "stage:in-progress" "stage:in-review" "stage:blocked" "stage:ready" "needs-human"; do
    echo "### $label"
    gh issue list --label "$label" --state open --limit 30 \
      --json number,title,milestone --jq '.[] | "- #\(.number) \(.title) [\(.milestone.title // "no milestone")]"' 2>/dev/null
  done
  echo "### Unlabeled-stage open issues"
  gh issue list --state open --limit 50 --json number,title,labels \
    --jq '.[] | select([.labels[].name | startswith("stage:")] | any | not) | "- #\(.number) \(.title)"' 2>/dev/null
  echo "### Open PRs"
  gh pr list --state open --json number,title,headRefName --jq '.[] | "- #\(.number) \(.title) (\(.headRefName))"' 2>/dev/null
  echo
  echo "## Latest sprint report"
  latest=$(ls docs/sprints/sprint-*.md 2>/dev/null | sort | tail -1)
  echo "- ${latest:-none}"
} >"$out"
test -s "$out"
cat "$out"
