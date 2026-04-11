# Release Notes Template

Use this template when you want more structured release notes than the default GitHub auto-generated release output.

## Recommended Flow

1. Prepare and validate the release locally.
2. Generate a draft:

```sh
./scripts/generate-release-notes.sh vX.Y.Z > /tmp/vX.Y.Z-release-notes.md
```

3. Replace the `TODO` bullets with a short user-facing summary.
4. Use the generated markdown in GitHub Release notes, release announcements, or changelog posts.

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
