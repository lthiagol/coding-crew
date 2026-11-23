# Release

A release is a version bump, a gate, and a tag. The taps build from the source
tarball of the tag, so no binaries are attached. Bottles are the maintainer's,
built with `bottle.sh`.

1. Bump `version` under `[workspace.package]` in the root `Cargo.toml`. Every
   crate inherits it, and `raul --version` and `hugo --version` print it.
2. Run `make check`. It must pass. CI runs the same gate on Linux and macOS.
3. Commit the bump: `chore: release vX.Y.Z`.
4. Tag and push: `git tag vX.Y.Z` then `git push origin vX.Y.Z`.
5. Run the `homebrew` skill to add or bump `coding-crew` in
   `lthiagol/homebrew-tap` from the tag.
