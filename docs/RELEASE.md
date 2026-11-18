# Publishing OSD

The three crates (`osd`, `osd-formats`, `osd-cli`) share one version, declared once in `[workspace.package]` of the workspace manifest. The release workflow (`.github/workflows/release.yml`) publishes them, the `osd` binaries for four platforms and the npm package `@roomi-fields/osd`.

## What starts it

- A `release` event on GitHub publishes. The release tag is `v<version>`, and the workflow refuses a tag that differs from the workspace version.
- A manual run (`workflow_dispatch`) has a `dry-run` input, true by default. A manual run never publishes: it runs the checks only.

## The order

1. **Checks**: the four binaries are built, `cargo publish --workspace --dry-run` and `npm publish --dry-run` pass, and the three install checks (cargo, a downloaded binary, npm) run on the artefacts, each checking `osd --version` against the version being released.
2. **Crates**: `osd`, then `osd-formats`, then `osd-cli`, on crates.io.
3. **Binaries**: each binary and one checksum file, `SHA256SUMS`, are attached to the release.
4. **npm**: `@roomi-fields/osd`, its version set from the release tag.
5. **Install checks again**, from the registries: `cargo install osd-cli`, the binary of the release against `SHA256SUMS`, `npm install -g @roomi-fields/osd`.

Every publication runs only on a `release` event, never on a dry run.

## What a failure does

- A failed check stops the workflow before anything is published.
- A failed crates.io step stops the binaries and the npm package: none is published alone.
- A failed install check after the publication leaves the workflow red and rolls nothing back. A published version is yanked only by the owner.

## Authentication

Publication uses trusted publishing (OIDC): `id-token: write` on the crates and npm jobs only, and no access token is kept in the repository.

## The first publication of each name

The first publication of each new name (`osd`, `osd-formats` and `osd-cli` on crates.io, `@roomi-fields/osd` on npm) is done by hand by the owner: trusted publishing needs the name to exist. Then the repository `roomi-fields/open-synth-descriptors` is declared trusted publisher of each name (workflow `release.yml`), on crates.io and on npm. From the next release, the workflow publishes.

The release workflow of this repository publishes nothing until a release is made: the first one is the owner's.
