#!/usr/bin/env bash

set -euo pipefail

usage() {
  cat <<'EOF'
Usage:
  ./scripts/prepare-release.sh <new-version>

Example:
  ./scripts/prepare-release.sh 2.0.3
EOF
}

if [ "${1:-}" = "" ] || [ "${1:-}" = "-h" ] || [ "${1:-}" = "--help" ]; then
  usage
  exit 0
fi

new_version="$1"

if [[ ! "$new_version" =~ ^[0-9]+\.[0-9]+\.[0-9]+([.-][0-9A-Za-z.-]+)?$ ]]; then
  echo "Invalid version: $new_version" >&2
  echo "Expected a semver-like value such as 2.0.3 or 2.1.0-rc.1" >&2
  exit 1
fi

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

current_version="$(sed -n 's/^version = "\(.*\)"/\1/p' src-rust/Cargo.toml | head -n1)"

if [ -z "$current_version" ]; then
  echo "Failed to detect current version from src-rust/Cargo.toml" >&2
  exit 1
fi

if [ "$current_version" = "$new_version" ]; then
  echo "Version is already $new_version"
  exit 0
fi

CURRENT_VERSION="$current_version" NEW_VERSION="$new_version" node <<'EOF'
const fs = require('fs');

const currentVersion = process.env.CURRENT_VERSION;
const newVersion = process.env.NEW_VERSION;

function replaceExact(filePath, replacer) {
  const original = fs.readFileSync(filePath, 'utf8');
  const updated = replacer(original);
  if (updated === original) {
    throw new Error(`No changes were applied to ${filePath}`);
  }
  fs.writeFileSync(filePath, updated);
}

replaceExact('src-rust/Cargo.toml', content =>
  content.replace(/^version = ".*"$/m, `version = "${newVersion}"`)
);

replaceExact('src-rust/Cargo.lock', content =>
  content.replace(
    /(name = "serve-here"\nversion = ")([^"]+)(")/,
    `$1${newVersion}$3`
  )
);

const pkgPath = 'npm/serve-here/package.json';
const pkg = JSON.parse(fs.readFileSync(pkgPath, 'utf8'));
pkg.version = newVersion;
for (const key of Object.keys(pkg.optionalDependencies || {})) {
  pkg.optionalDependencies[key] = newVersion;
}
fs.writeFileSync(pkgPath, `${JSON.stringify(pkg, null, 2)}\n`);

replaceExact('README.md', content => {
  let updated = content;
  updated = updated.replace(`> **v${currentVersion}**:`, `> **v${newVersion}**:`);
  updated = updated.replace(`> **v${currentVersion}**：`, `> **v${newVersion}**：`);
  updated = updated.replace(`git tag v${currentVersion}`, `git tag v${newVersion}`);
  updated = updated.replace(`git push origin v${currentVersion}`, `git push origin v${newVersion}`);
  return updated;
});
EOF

echo "Updated release version: $current_version -> $new_version"
echo
echo "Next suggested steps:"
echo "  cargo test --locked --manifest-path src-rust/Cargo.toml"
echo "  npx playwright test e2e/webui.spec.js --reporter=line"
echo "  git add README.md src-rust/Cargo.toml src-rust/Cargo.lock npm/serve-here/package.json"
echo "  git commit -m \"release: prepare v${new_version}\""
echo "  git tag v${new_version}"
echo "  git push origin main"
echo "  git push origin v${new_version}"
