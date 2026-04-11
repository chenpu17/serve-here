#!/usr/bin/env bash

set -euo pipefail

usage() {
  cat <<'EOF'
Usage:
  ./scripts/release-all.sh <new-version> [options]

Options:
  --dry-run              Print the release steps without executing them
  --no-push              Do everything except pushing the branch and tag
  --notes-file <path>    Write release notes draft to the given path
  -h, --help             Show this help message

Examples:
  ./scripts/release-all.sh 2.0.3
  ./scripts/release-all.sh 2.0.3 --no-push
  ./scripts/release-all.sh 2.0.3 --dry-run
EOF
}

if [ $# -eq 0 ]; then
  usage
  exit 0
fi

new_version=""
dry_run=false
push_enabled=true
notes_file=""

while [ $# -gt 0 ]; do
  case "$1" in
    --dry-run)
      dry_run=true
      shift
      ;;
    --no-push)
      push_enabled=false
      shift
      ;;
    --notes-file)
      if [ $# -lt 2 ]; then
        echo "Missing value for --notes-file" >&2
        exit 1
      fi
      notes_file="$2"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    -*)
      echo "Unknown option: $1" >&2
      exit 1
      ;;
    *)
      if [ -n "$new_version" ]; then
        echo "Unexpected argument: $1" >&2
        exit 1
      fi
      new_version="$1"
      shift
      ;;
  esac
done

if [ -z "$new_version" ]; then
  echo "Missing version argument" >&2
  exit 1
fi

if [[ ! "$new_version" =~ ^[0-9]+\.[0-9]+\.[0-9]+([.-][0-9A-Za-z.-]+)?$ ]]; then
  echo "Invalid version: $new_version" >&2
  echo "Expected a semver-like value such as 2.0.3 or 2.1.0-rc.1" >&2
  exit 1
fi

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

if ! git diff --quiet || ! git diff --cached --quiet; then
  echo "Working tree must be clean before running release-all.sh" >&2
  exit 1
fi

current_branch="$(git rev-parse --abbrev-ref HEAD)"
version_tag="v${new_version}"

if git rev-parse -q --verify "refs/tags/${version_tag}" >/dev/null; then
  echo "Tag already exists: ${version_tag}" >&2
  exit 1
fi

if [ -z "$notes_file" ]; then
  notes_file="/tmp/${version_tag}-release-notes.md"
fi

run_cmd() {
  printf '+'
  for arg in "$@"; do
    printf ' %q' "$arg"
  done
  printf '\n'

  if [ "$dry_run" = false ]; then
    "$@"
  fi
}

echo "Release plan"
echo "  version: ${new_version}"
echo "  branch:  ${current_branch}"
echo "  notes:   ${notes_file}"
echo "  push:    ${push_enabled}"
echo "  dry-run: ${dry_run}"
echo

run_cmd ./scripts/prepare-release.sh "$new_version"
run_cmd cargo test --locked --manifest-path src-rust/Cargo.toml
run_cmd npx playwright test e2e/webui.spec.js --reporter=line

if [ "$dry_run" = false ]; then
  ./scripts/generate-release-notes.sh "$version_tag" > "$notes_file"
else
  printf '+ %q %q > %q\n' ./scripts/generate-release-notes.sh "$version_tag" "$notes_file"
fi

run_cmd git add README.md src-rust/Cargo.toml src-rust/Cargo.lock npm/serve-here/package.json
run_cmd git commit -m "release: prepare ${version_tag}"
run_cmd git tag "$version_tag"

if [ "$push_enabled" = true ]; then
  run_cmd git push origin "$current_branch"
  run_cmd git push origin "$version_tag"
else
  echo "Push skipped by --no-push"
fi

echo
echo "Release orchestration complete."
echo "Notes draft: ${notes_file}"
