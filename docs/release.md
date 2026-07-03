# Release Automation

Releases are tag-driven. Push a version tag such as `v0.1.0` to run the release
workflow:

```sh
git tag v0.1.0
git push origin v0.1.0
```

The release workflow:

1. runs the same governance, format, clippy, test, release-build, and whitespace
   gates used by CI;
2. builds a universal macOS archive and a Linux x86_64 archive;
3. creates a GitHub Release with checksums;
4. updates the `xzhih/tap` Homebrew tap, backed by
   `xzhih/homebrew-tap`, with a formula pointing at the macOS release archive.

The repository needs a GitHub Actions secret named `TAP_DEPLOY_KEY`. Store a
private SSH key that has write access to
`git@github.com:xzhih/homebrew-tap.git`. The matching public key should be added
to the tap repository as a deploy key with write access, or the private key
should belong to a machine user that can push to the tap repository.

After a successful release, macOS users can install with:

```sh
brew install xzhih/tap/icon-tracer
```
