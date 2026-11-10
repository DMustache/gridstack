#!/usr/bin/env bash
# Task board helper. Labels are the source of truth; the project board mirrors them.
#
#   task.sh create <title> <body-file> <milestone> <labels-csv> [sprint]   -> prints issue number
#   task.sh stage  <issue> <backlog|ready|in-progress|in-review|blocked|done>
#   task.sh blocker <blocked-issue> <title> <body-file>                    -> prints blocker number
#   task.sh sprint <issue> <sprint-number>
set -euo pipefail
cd "$(dirname "$0")/../.."
[ -f .github/agents-project.env ] && source .github/agents-project.env

stage_name() {
  case "$1" in
    backlog) echo Backlog ;; ready) echo Ready ;; in-progress) echo "In progress" ;;
    in-review) echo "In review" ;; done) echo Done ;;
    blocked) echo "" ;; # board has no Blocked column: label stage:blocked only
    *) echo "unknown stage $1" >&2; exit 2 ;;
  esac
}

project_item_id() {
  local issue_url
  issue_url=$(gh issue view "$1" --json url --jq .url)
  gh project item-add "$PROJECT_NUMBER" --owner "$PROJECT_OWNER" --url "$issue_url" --format json --jq .id
}

set_project_field() { # issue field value(single-select name | number)
  [ -n "${PROJECT_NUMBER:-}" ] || return 0
  local item project_id field_json field_id option_id
  item=$(project_item_id "$1")
  project_id=$(gh project view "$PROJECT_NUMBER" --owner "$PROJECT_OWNER" --format json --jq .id)
  field_json=$(gh project field-list "$PROJECT_NUMBER" --owner "$PROJECT_OWNER" --format json)
  field_id=$(jq -r --arg n "$2" '.fields[] | select(.name==$n) | .id' <<<"$field_json")
  option_id=$(jq -r --arg n "$2" --arg v "$3" '.fields[] | select(.name==$n) | (.options // [])[] | select(.name==$v) | .id' <<<"$field_json")
  if [ -n "$option_id" ]; then
    gh project item-edit --id "$item" --project-id "$project_id" --field-id "$field_id" --single-select-option-id "$option_id" >/dev/null
  else
    gh project item-edit --id "$item" --project-id "$project_id" --field-id "$field_id" --number "$3" >/dev/null
  fi
}

set_stage() {
  local current
  current=$(gh issue view "$1" --json labels --jq '[.labels[].name | select(startswith("stage:"))] | join(",")')
  if [ "$2" = done ]; then
    [ -n "$current" ] && gh issue edit "$1" --remove-label "$current" >/dev/null
  else
    gh issue edit "$1" ${current:+--remove-label "$current"} --add-label "stage:$2" >/dev/null
  fi
  local status
  status=$(stage_name "$2")
  [ -n "$status" ] || return 0
  set_project_field "$1" Status "$status" || echo "warn: project board not updated" >&2
}

command="${1:-}"; shift || true
case "$command" in
  create)
    url=$(gh issue create --title "$1" --body-file "$2" --milestone "$3" --label "task,$4")
    number="${url##*/}"
    set_stage "$number" ready
    [ -n "${5:-}" ] && set_project_field "$number" Sprint "$5"
    case ",$4," in *,size:s,*) set_project_field "$number" Size S ;; *,size:m,*) set_project_field "$number" Size M ;; esac
    echo "$number"
    ;;
  stage) set_stage "$1" "$2" ;;
  sprint) set_project_field "$1" Sprint "$2" ;;
  blocker)
    url=$(gh issue create --title "[blocker] $2" --body-file "$3" --label blocker)
    number="${url##*/}"
    gh issue comment "$1" --body "Blocked by #$number" >/dev/null
    set_stage "$1" blocked
    echo "$number"
    ;;
  *) sed -n '2,8p' "$0"; exit 2 ;;
esac
