# Releasing

`health-dsl` ships three native implementations from one repo. Each has its own
release channel, triggered by a **language-prefixed git tag**. Pushing a tag
runs the matching GitHub Actions workflow in [`.github/workflows`](.github/workflows).

| Implementation | Tag prefix | Workflow                | Target                                  |
|----------------|------------|-------------------------|-----------------------------------------|
| Rust           | `rust-v*`  | `release-rust.yml`      | [crates.io](https://crates.io/crates/health-dsl) |
| TypeScript     | `npm-v*`   | `release-npm.yml`       | [npm](https://www.npmjs.com/package/@slothlabs/health-dsl) |
| Kotlin / JVM   | `jvm-v*`   | `release-maven.yml`     | GitHub Packages (`com.slothlabs:health-dsl`) |

Example — cut a Rust 0.2.0 release:

```sh
# bump version in rust/Cargo.toml first, commit, then:
git tag -a rust-v0.2.0 -m "rust-v0.2.0"
git push origin rust-v0.2.0
```

The `v0.1.0`-style umbrella tag is used by **JitPack** for the JVM artifact and
as a human-facing release marker; it does not trigger any publish workflow.

## One-time setup

### Rust → crates.io
1. Create a crates.io API token (Account Settings → API Tokens).
2. Add it as the repo secret **`CARGO_REGISTRY_TOKEN`**
   (Settings → Secrets and variables → Actions).
3. The crate name `health-dsl` must be owned by your crates.io account.

### TypeScript → npm
1. You must own/control the **`@slothlabs`** npm org/scope. Create it at
   npmjs.com if it does not exist, and make sure the publishing account is a
   member with publish rights.
2. Create an **automation** npm token (npmjs.com → Access Tokens →
   Granular/Automation, with publish access to `@slothlabs/*`).
3. Add it as the repo secret **`NPM_TOKEN`**.
4. The workflow publishes with `--provenance`, which requires the
   `id-token: write` permission (already set in `release-npm.yml`) and a
   public repo or appropriately configured org.

### Kotlin / JVM
Two distribution paths, no extra setup required for either:

- **JitPack (zero-config):** consumers add `https://jitpack.io` and depend on
  `com.github.slothlabsorg:health-dsl:<tag>`. JitPack builds from the git tag
  using [`jitpack.yml`](jitpack.yml) — **only a git tag is needed**, no secrets,
  no manual publish.
- **GitHub Packages:** the `jvm-v*` workflow publishes
  `com.slothlabs:health-dsl` using the built-in `GITHUB_TOKEN` (the workflow
  declares `permissions: packages: write`). No additional secret to configure.

#### Optional: Maven Central
If you later want `com.slothlabs:health-dsl` on Maven Central, you'll
additionally need: a verified `com.slothlabs` namespace on Central (Sonatype),
a GPG signing key, and the `signing` + a Central-publishing plugin wired into
`kotlin/build.gradle.kts`. Not configured today.

## Local validation before tagging

```sh
# Rust
cargo build                                   # from repo root (workspace)
( cd rust && cargo test && cargo clippy --all-targets -- -D warnings && cargo fmt --all --check )

# TypeScript
( cd ts && npm ci && npm run typecheck && npm test && npm run build && npm publish --dry-run )

# Kotlin / JVM
( cd kotlin && gradle build && gradle publishToMavenLocal -Pversion=0.1.0 )
```
