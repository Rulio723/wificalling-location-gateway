#!/bin/sh
set -eu

if [ "$#" -ne 1 ]; then
    echo "usage: $0 <issue-number>" >&2
    exit 2
fi

issue=$1
case "$issue" in *[!0-9]*|'') echo 'issue-number must be numeric' >&2; exit 2 ;; esac

gh issue view "$issue" --json state,labels --jq '.state' | grep -qx OPEN || {
    echo "issue #$issue is not open" >&2
    exit 1
}

gh issue edit "$issue" --add-assignee '@me' --remove-label 'status:ready' --add-label 'status:claimed'
gh issue comment "$issue" --body "Claimed for isolated Agent development. Branch: \`codex/issue-$issue-<slug>\`."
