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
- A failed crates.io step stops the binaries and the npm package: none is published alone. A release run again publishes only what is missing.
- A failed install check after the publication leaves the workflow red and rolls nothing back. A published version is yanked only by the owner.

## Authentication

Publication uses trusted publishing (OIDC): `id-token: write` on the crates and npm jobs only, and no access token is kept in the repository.

## The first release

Trusted publishing needs a name to exist, and the npm package installs the binaries of its release, so the first release goes in this order:

1. The owner publishes the three crates by hand (`cargo publish -p osd`, `-p osd-formats`, `-p osd-cli`) and declares the repository `roomi-fields/open-synth-descriptors` trusted publisher of each (workflow `release.yml`).
2. The release `v<version>` is made. The workflow finds the three crates on crates.io and skips their publication, saying so in the log; it attaches the binaries. The npm job finds no `@roomi-fields/osd` on npm and skips, saying so; the install checks run, the npm one only when the package is on the registry.
3. Once the binaries are on the release, the owner publishes `@roomi-fields/osd` by hand from `npm/`, and declares the trusted publisher on npm.
4. From the next release, the workflow publishes everything.

## What a publication skips

Each publication first asks its registry whether the version is there, and skips it when it is, saying so in the log. crates.io: `/api/v1/crates/<name>/<version>`; only a 404 means absent, and a yanked version counts as present. npm: `npm view <name>@<version>`; only an E404 means absent. Any other answer (network, 5xx, rate limit) fails the step: nothing is skipped on a doubt. A name absent from the registry skips its publication (the first one is by hand). So a release run again after a partial failure publishes only what is missing. The authentication to crates.io runs only when a crate is to be published, and the binaries run when the crates job succeeded, whether it published or skipped.

A manual run publishes nothing.
