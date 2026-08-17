# Releasing

## Cutting a release

1. Move the `Unreleased` entries in [`CHANGELOG.md`](CHANGELOG.md) under a new
   version heading, and add the comparison links at the bottom of that file.
2. Bump `version` in `Cargo.toml`, run `cargo test`, and commit.
3. Tag and push:

   ```sh
   git tag -a v0.1.0 -m "v0.1.0"
   git push origin v0.1.0
   ```

4. The `Release` workflow builds `hl7test` for macOS (Apple silicon and Intel)
   and Linux (x86_64 and aarch64), then attaches the tarballs and their SHA-256
   sums to a GitHub release.

To rebuild an existing tag, run the workflow manually from the Actions tab and
pass the tag name.

## Updating the Homebrew formula

The formula builds from the tagged source tarball, so it only needs the new URL
and checksum:

```sh
version=0.1.0
url="https://github.com/sudhi001/hl7probe/archive/refs/tags/v${version}.tar.gz"
sha=$(curl -sL "$url" | shasum -a 256 | cut -d' ' -f1)
echo "$url"
echo "$sha"
```

Put those two values into `Formula/hl7probe.rb` here and in the copy published in
the [`sudhi001/homebrew-tap`](https://github.com/sudhi001/homebrew-tap)
repository, then verify locally:

```sh
brew install --build-from-source ./Formula/hl7probe.rb
brew test hl7probe
brew audit --strict --new hl7probe
```

Users install with:

```sh
brew install sudhi001/tap/hl7probe
```

The bare `brew install hl7probe` needs the formula to live in homebrew-core,
which accepts a project once it is established; until then the tap prefix, or a
one-off `brew tap sudhi001/tap`, is required.
