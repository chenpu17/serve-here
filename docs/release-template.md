# Release Notes Template

Use this template when you want more structured release notes than the default GitHub auto-generated release output.

## Recommended Flow

1. Prepare the version update locally.

```sh
./scripts/prepare-release.sh X.Y.Z
```

2. Validate the release locally.

```sh
cargo test --locked --manifest-path src-rust/Cargo.toml
npx playwright test e2e/webui.spec.js --reporter=line
```

3. Generate a draft:

```sh
./scripts/generate-release-notes.sh vX.Y.Z > /tmp/vX.Y.Z-release-notes.md
```

4. Replace the `TODO` bullets with a short user-facing summary.
5. Use the generated markdown in GitHub Release notes, release announcements, or changelog posts.

If you want the whole local release flow in one command, you can also run:

```sh
./scripts/release-all.sh X.Y.Z --no-push
```

## Suggested Structure

```md
# vX.Y.Z

Released: YYYY-MM-DD

## Highlights

- The most important change.

## User-Facing Changes

- UI changes
- CLI changes
- Packaging or platform changes

## Quality And Verification

- cargo test --locked
- npx playwright test e2e/webui.spec.js --reporter=line

## npm Packages

- @chenpu17/serve-here@X.Y.Z
- Platform packages...

## Compare

- https://github.com/<owner>/<repo>/compare/vX.Y.(Z-1)...vX.Y.Z

## Included Commits

- Short commit summary list
```

## Notes

- Keep the Highlights section short. Three bullets is enough.
- Prefer user-visible language over implementation details.
- If a release only changes docs or pipeline internals, say that directly.
- The GitHub Release workflow can publish notes from the generated markdown file instead of relying only on GitHub auto-generated notes.
