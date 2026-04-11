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
target_ref="${3:-}"
version="${version_tag#v}"
release_date="$(date +%F)"

if [ -z "$target_ref" ]; then
  if git rev-parse -q --verify "refs/tags/${version_tag}" >/dev/null; then
    target_ref="${version_tag}"
  else
    target_ref="HEAD"
  fi
fi

if [ -z "$previous_tag" ]; then
  previous_tag="$(
    git tag --sort=version:refname | awk -v target="$version_tag" '
      { tags[++count] = $0 }
      END {
        for (i = 1; i <= count; i++) {
          if (tags[i] == target) {
            if (i > 1) {
              print tags[i - 1]
            }
            exit
          }
        }
      }
    '
  )"

  if [ -z "$previous_tag" ]; then
    previous_tag="$(git tag --sort=-version:refname | grep -Fxv "$version_tag" | head -n1 || true)"
  fi
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

commit_lines="$(git log --reverse --pretty=format:'- %h %s' "$log_range" || true)"
commit_subjects="$(git log --reverse --pretty=format:'%h%x09%s' "$log_range" || true)"

sections_dir="$(mktemp -d)"
trap 'rm -rf "$sections_dir"' EXIT

COMMIT_SUBJECTS="$commit_subjects" node - "$sections_dir" <<'EOF'
const fs = require('fs');
const outDir = process.argv[2];
const input = (process.env.COMMIT_SUBJECTS || '').trim();

const commits = input
  ? input.split('\n').map(line => {
      const [hash, ...rest] = line.split('\t');
      return { hash, subject: rest.join('\t').trim() };
    })
  : [];

function unique(items) {
  return [...new Set(items.filter(Boolean))];
}

function sentence(text) {
  const trimmed = text.trim();
  if (!trimmed) return '';
  const capitalized = trimmed.charAt(0).toUpperCase() + trimmed.slice(1);
  return /[.!?]$/.test(capitalized) ? capitalized : `${capitalized}.`;
}

function describeCommit(subject) {
  const lower = subject.toLowerCase();

  const knownPatterns = [
    [/release notes generator/, 'Added a reusable release notes generator for consistent release drafts.'],
    [/release prep script/, 'Added a release preparation script to sync version bumps across Rust, npm, and README references.'],
    [/release orchestration script/, 'Added a one-command release orchestration script that prepares, validates, tags, and publishes a release.'],
    [/publish generated release notes/, 'GitHub Releases now publish repository-generated notes instead of relying only on default auto-generated notes.'],
    [/dark theme screenshots/, 'README now includes dark theme screenshots for the web UI.'],
    [/quick start guide/, 'README quick start guidance was expanded to make first-run setup easier.'],
  ];

  for (const [pattern, description] of knownPatterns) {
    if (pattern.test(lower)) {
      return description;
    }
  }

  const match = subject.match(/^([^:]+):\s*(.*)$/);
  if (!match) {
    return sentence(subject);
  }

  const kind = match[1].toLowerCase();
  const body = match[2];

  if (kind === 'release' && /^prepare v/i.test(body)) {
    return '';
  }

  if (kind === 'docs') {
    return `Documentation: ${sentence(body)}`;
  }

  if (kind === 'build' || kind === 'ci') {
    return `Tooling: ${sentence(body)}`;
  }

  if (kind === 'feat') {
    return sentence(body);
  }

  if (kind === 'fix') {
    return `Fixed ${body.replace(/^[a-z]/, ch => ch.toLowerCase())}.`;
  }

  return sentence(body);
}

function classifyCommit(subject) {
  const lower = subject.toLowerCase();
  const prefix = (subject.match(/^([^:]+):/) || [])[1]?.toLowerCase() || '';

  if (prefix === 'release' && lower.includes('prepare v')) {
    return 'meta';
  }

  if (
    prefix === 'feat' ||
    prefix === 'fix' ||
    /\b(ui|webui|listing|stats|theme|lang|bilingual|browser|dashboard|server|cli)\b/.test(lower)
  ) {
    return 'user';
  }

  if (prefix === 'docs') {
    return 'docs';
  }

  if (prefix === 'build' || prefix === 'ci') {
    return 'tooling';
  }

  return 'other';
}

const buckets = {
  user: [],
  docs: [],
  tooling: [],
  other: [],
};

const commitBuckets = {
  user: [],
  docs: [],
  tooling: [],
  other: [],
};

for (const commit of commits) {
  const bucket = classifyCommit(commit.subject);
  if (bucket === 'meta') continue;
  commitBuckets[bucket].push(commit);
  buckets[bucket].push(describeCommit(commit.subject));
}

const userChanges = unique(buckets.user);
const toolingChanges = unique([...buckets.tooling, ...buckets.docs, ...buckets.other]);

let highlights = [];
let changes = [];

if (userChanges.length > 0) {
  highlights = unique([
    ...userChanges.slice(0, 2),
    toolingChanges[0] || '',
  ]).slice(0, 3);

  changes = unique([
    ...userChanges,
    ...toolingChanges.slice(0, 3),
  ]);
} else if (toolingChanges.length > 0) {
  const hasPrep = toolingChanges.some(line => /preparation script/i.test(line));
  const hasOrchestration = toolingChanges.some(line => /orchestration script/i.test(line));
  const hasGeneratedNotes = toolingChanges.some(line => /repository-generated notes/i.test(line));
  const hasDocs = toolingChanges.some(line => /README|Documentation:/i.test(line));

  if (hasPrep || hasOrchestration || hasGeneratedNotes) {
    highlights.push('Release automation now covers version preparation, validation, tagging, publishing, and generated GitHub Release notes.');
  }

  if (hasDocs) {
    highlights.push('Release documentation and operator guidance are now more standardized and easier to reuse.');
  }

  highlights.push('This release focuses on delivery workflow improvements rather than runtime behavior changes.');
  highlights = unique(highlights).slice(0, 3);

  changes = unique([
    'No runtime behavior changes in this release; the focus is on release tooling, release notes, and documentation.',
    ...toolingChanges,
  ]);
} else {
  highlights = ['This release is mainly a maintenance update with no major user-facing changes.'];
  changes = ['No notable user-facing changes were detected from the commit range.'];
}

fs.writeFileSync(`${outDir}/highlights.md`, `${highlights.map(item => `- ${item}`).join('\n')}\n`);
fs.writeFileSync(`${outDir}/changes.md`, `${changes.map(item => `- ${item}`).join('\n')}\n`);

const groupedCommitCounts = [
  ['user', 'User-facing', commitBuckets.user.length],
  ['tooling', 'Tooling and CI', commitBuckets.tooling.length],
  ['docs', 'Documentation', commitBuckets.docs.length],
  ['other', 'Other', commitBuckets.other.length],
].filter(([, , count]) => count > 0);

const hasOnlyNonUserCommits =
  commitBuckets.user.length === 0 &&
  (commitBuckets.docs.length > 0 || commitBuckets.tooling.length > 0 || commitBuckets.other.length > 0);

let commitsBlock = '';

if (commits.length === 0) {
  commitsBlock = '- TODO: no commits found in the selected range.\n';
} else if (hasOnlyNonUserCommits) {
  commitsBlock = groupedCommitCounts
    .map(([, label, count]) => `- ${label}: ${count} commit${count === 1 ? '' : 's'}`)
    .join('\n');
  commitsBlock += '\n';
} else {
  const sections = [
    ['User-facing', commitBuckets.user],
    ['Tooling and CI', commitBuckets.tooling],
    ['Documentation', commitBuckets.docs],
    ['Other', commitBuckets.other],
  ].filter(([, items]) => items.length > 0);

  commitsBlock = sections
    .map(([label, items]) => {
      const lines = items.map(commit => `- ${commit.hash} ${commit.subject}`).join('\n');
      return `### ${label}\n\n${lines}`;
    })
    .join('\n\n');
  commitsBlock += '\n';
}

fs.writeFileSync(`${outDir}/commits.md`, commitsBlock);
EOF

highlights_block="$(cat "${sections_dir}/highlights.md")"
changes_block="$(cat "${sections_dir}/changes.md")"
commits_block="$(cat "${sections_dir}/commits.md")"

cat <<EOF
# ${version_tag}

Released: ${release_date}

## Highlights

${highlights_block}

## User-Facing Changes

${changes_block}

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

${commits_block}
EOF
