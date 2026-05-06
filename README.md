# block-detached-commit

[![CI](https://github.com/tupe12334/block-detached-commit/actions/workflows/ci.yml/badge.svg)](https://github.com/tupe12334/block-detached-commit/actions)
[![Crates.io](https://img.shields.io/crates/v/block-detached-commit)](https://crates.io/crates/block-detached-commit)
[![npm](https://img.shields.io/npm/v/block-detached-commit)](https://www.npmjs.com/package/block-detached-commit)

Git pre-commit hook that blocks commits in [detached HEAD](https://git-scm.com/docs/git-checkout#_detached_head) state.

## Why

When `HEAD` points directly to a commit SHA instead of a branch ref, any new commits you make become orphaned — not reachable from any branch. Git will garbage-collect them. Switching branches silently abandons your work.

This hook exits with a non-zero status before the commit lands, giving you a clear error message instead of a silent data-loss footgun.

## Install

### npm / npx

```bash
# one-time: installs hook into current repo
npx block-detached-commit install
```

```bash
# or install globally and run manually per repo
npm install -g block-detached-commit
block-detached-commit install
```

For projects using [Husky](https://typicode.github.io/husky) or [lint-staged](https://github.com/okonet/lint-staged), add to your `package.json`:

```json
{
  "scripts": {
    "prepare": "block-detached-commit install"
  }
}
```

### Go

```bash
go install github.com/tupe12334/block-detached-commit/go@latest
block-detached-commit install
```

### Cargo (crates.io)

```bash
cargo install block-detached-commit
block-detached-commit install
```

## How it works

The binary reads `.git/HEAD`. If the file contains a bare commit SHA (detached HEAD), it prints an error and exits `1`, aborting the commit. If it starts with `ref:`, HEAD is attached to a branch and the hook exits `0`.

```
HEAD content          State             Hook result
─────────────────────────────────────────────────
ref: refs/heads/main  attached branch   ✓ allow
a3f9c2d1...           detached HEAD     ✗ block
```

### The `install` subcommand

`block-detached-commit install` appends a call to the binary into `.git/hooks/pre-commit` of the current repository (creating the file if it doesn't exist and marking it executable). It is idempotent — running it twice does not duplicate the entry.

## Manual hook setup

If you manage hooks manually:

```bash
# .git/hooks/pre-commit
#!/bin/sh
block-detached-commit
```

Or inline without installing the binary at all:

```sh
#!/bin/sh
if ! git symbolic-ref HEAD > /dev/null 2>&1; then
  echo "error: cannot commit in detached HEAD state" >&2
  exit 1
fi
```

## Uninstall

```bash
block-detached-commit uninstall   # removes hook entry from current repo
```

Or delete `.git/hooks/pre-commit` manually if that was the only hook.

## Platform support

Pre-built binaries are published for:

| OS      | Architecture |
|---------|-------------|
| Linux   | x86_64, aarch64 |
| macOS   | x86_64, aarch64 (Apple Silicon) |
| Windows | x86_64 |

The npm and Go shells download the correct binary for the current platform on install. The Rust crate compiles from source via `cargo`.

## License

MIT
