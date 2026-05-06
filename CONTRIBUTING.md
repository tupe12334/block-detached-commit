# Contributing

## Project structure

```
block-detached-commit/
├── src/
│   └── main.rs          # Rust binary — all core logic lives here
├── Cargo.toml           # crates.io package definition
├── npm/
│   ├── package.json     # npm package (shell)
│   ├── bin/
│   │   └── cli.js       # entry point: delegates to native binary
│   └── scripts/
│       └── postinstall.js  # downloads correct platform binary on npm install
├── go/
│   ├── go.mod           # Go module (shell)
│   └── main.go          # downloads + runs native binary
└── .github/
    └── workflows/
        ├── ci.yml       # test on push/PR
        └── release.yml  # build cross-platform, publish to npm/crates.io
```

**Rule:** business logic (detecting detached HEAD, modifying the hook file) lives only in `src/main.rs`. The npm and Go packages are thin shells — their only job is to locate and run the native binary.

## Dev setup

Prerequisites: Rust stable, Node.js ≥18, Go ≥1.21.

```bash
git clone https://github.com/tupe12334/block-detached-commit
cd block-detached-commit

# build the Rust binary
cargo build

# run directly (no install)
./target/debug/block-detached-commit
```

## Binary interface

The binary is the contract between the Rust core and the shell wrappers.

```
USAGE:
    block-detached-commit             # check; exit 0 = ok, exit 1 = detached
    block-detached-commit install     # add hook to .git/hooks/pre-commit
    block-detached-commit uninstall   # remove hook entry
```

All user-facing output goes to stderr. Stdout is silent (makes it safe to use in scripts).

Exit codes:

| Code | Meaning |
|------|---------|
| 0    | HEAD is attached / hook installed successfully |
| 1    | Detached HEAD detected / hook not present |
| 2    | Not inside a git repository |

Do not change exit codes or subcommand names without a major version bump — the shells depend on them.

## Rust core (`src/main.rs`)

The detached HEAD check reads `.git/HEAD` directly (no `git` process spawn) for speed. Falls back to `git symbolic-ref HEAD` if the file read fails (worktrees, unusual setups).

```bash
cargo test          # unit tests
cargo clippy        # lint
cargo fmt --check   # format
```

## npm shell (`npm/`)

The shell resolves the binary path at runtime:

1. Check `node_modules/.bin/block-detached-commit-<platform>` (optional peer dep)
2. Check `PATH`
3. Error with install instructions

```bash
cd npm
npm install
node bin/cli.js         # run locally
node scripts/postinstall.js  # simulate postinstall
```

Platform packages follow the naming scheme: `block-detached-commit-<os>-<arch>` (e.g. `block-detached-commit-linux-x64`). These are published separately and listed as `optionalDependencies` in `npm/package.json`.

## Go shell (`go/`)

```bash
cd go
go build ./...
go test ./...
```

The Go shell downloads the binary from the GitHub release matching `runtime.GOOS`/`runtime.GOARCH` and caches it in the user's cache dir (`os.UserCacheDir()`). It verifies the SHA-256 checksum published alongside each release asset.

## Cross-compilation and releases

Releases are driven by `release.yml`. On a version tag (`v*`):

1. Builds the Rust binary for all platform targets via cross (Docker-based cross-compilation).
2. Uploads binaries as GitHub release assets with SHA-256 checksums.
3. Publishes to crates.io via `cargo publish`.
4. Updates `npm/package.json` version and publishes platform packages, then the meta-package.

**Supported targets:**

| Target triple                  | npm os/cpu          |
|-------------------------------|---------------------|
| `x86_64-unknown-linux-gnu`    | linux / x64         |
| `aarch64-unknown-linux-gnu`   | linux / arm64       |
| `x86_64-apple-darwin`         | darwin / x64        |
| `aarch64-apple-darwin`        | darwin / arm64      |
| `x86_64-pc-windows-msvc`      | win32 / x64         |

To add a new target: add it to the matrix in `release.yml`, add the corresponding platform package in `npm/`, and add the GOOS/GOARCH case in `go/main.go`.

## Testing the hook end-to-end

```bash
# create a scratch repo in detached HEAD
tmp=$(mktemp -d)
git init "$tmp" && cd "$tmp"
git commit --allow-empty -m "init"
git checkout --detach HEAD

# install the hook
block-detached-commit install

# attempt a commit — should be blocked
git commit --allow-empty -m "should fail"
# expected: "error: cannot commit in detached HEAD state"
```

## PR checklist

- [ ] New behavior has a test in `src/main.rs`
- [ ] `cargo clippy` passes with no warnings
- [ ] `cargo fmt` applied
- [ ] Binary interface contract unchanged (or major version bumped)
- [ ] CHANGELOG entry added

## Releasing

Maintainers only. Bump the version consistently across `Cargo.toml`, `npm/package.json`, and `go/go.mod`, then push a tag:

```bash
git tag v1.2.3
git push origin v1.2.3
```

CI handles the rest.
