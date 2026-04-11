#!/usr/bin/env bash

set -euo pipefail

usage() {
  cat <<'EOF'
Usage:
  ./scripts/generate-release-notes.sh <version-tag> [previous-tag] [target-ref]

Examples:
  ./scripts/generate-release-notes.sh v2.0.3
  ./scripts/generate-release-notes.sh v2.0.3 v2.0.2
  ./scripts/generate-release-notes.sh v2.0.3 v2.0.2 HEAD
EOF
}

if [ "${1:-}" = "" ] || [ "${1:-}" = "-h" ] || [ "${1:-}" = "--help" ]; then
  usage
  exit 0
fi

version_tag="$1"
previous_tag="${2:-}"
target_ref="${3:-HEAD}"
version="${version_tag#v}"
release_date="$(date +%F)"

if [ -z "$previous_tag" ]; then
  previous_tag="$(git tag --sort=-version:refname | grep -Fxv "$version_tag" | head -n1 || true)"
fi

remote_url="$(git config --get remote.origin.url || true)"
repo_url=""

if [[ "$remote_url" =~ ^git@github\.com:(.+)\.git$ ]]; then
  repo_url="https://github.com/${BASH_REMATCH[1]}"
elif [[ "$remote_url" =~ ^https://github\.com/(.+)\.git$ ]]; then
  repo_url="https://github.com/${BASH_REMATCH[1]}"
elif [[ "$remote_url" =~ ^https://github\.com/(.+)$ ]]; then
  repo_url="https://github.com/${BASH_REMATCH[1]}"
fi

if [ -n "$previous_tag" ]; then
  log_range="${previous_tag}..${target_ref}"
else
  log_range="${target_ref}"
fi

commit_lines="$(git log --reverse --pretty=format:'- %h %s' "$log_range")"

cat <<EOF
# ${version_tag}

Released: ${release_date}

## Highlights

- TODO: summarize the most important user-facing improvements in 1-3 bullets.

## User-Facing Changes

- TODO: mention UI, behavior, compatibility, or CLI improvements.

## Quality And Verification

- Verified with \`cargo test --locked\`
- Verified with \`npx playwright test e2e/webui.spec.js --reporter=line\`

## npm Packages

- \`@chenpu17/serve-here@${version}\`
- \`@chenpu17/serve-here-linux-x64@${version}\`
- \`@chenpu17/serve-here-linux-arm64@${version}\`
- \`@chenpu17/serve-here-darwin-x64@${version}\`
- \`@chenpu17/serve-here-darwin-arm64@${version}\`
- \`@chenpu17/serve-here-windows-x64@${version}\`
EOF

if [ -n "$repo_url" ] && [ -n "$previous_tag" ]; then
  cat <<EOF

## Compare

- ${repo_url}/compare/${previous_tag}...${version_tag}
EOF
fi

cat <<EOF

## Included Commits

${commit_lines:-"- TODO: no commits found in the selected range."}
EOF
