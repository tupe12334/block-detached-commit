#!/usr/bin/env bash
set -euo pipefail

VERSION="${1:-}"
if [[ -z "$VERSION" ]]; then
  echo "Usage: $0 <version>  (e.g. 0.1.4)" >&2
  exit 1
fi

# Strip leading 'v' so we store bare semver in files
BARE="${VERSION#v}"
TAG="v${BARE}"

# Validate semver
if ! [[ "$BARE" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
  echo "Error: '$BARE' is not valid semver (MAJOR.MINOR.PATCH)" >&2
  exit 1
fi

ROOT="$(cd "$(dirname "$0")/.." && pwd)"

echo "Bumping to $TAG..."

# Cargo.toml — replace version = "x.y.z" on the first occurrence (package section)
sed -i.bak -E "s/^version = \"[0-9]+\.[0-9]+\.[0-9]+\"/version = \"${BARE}\"/" "$ROOT/Cargo.toml"
rm "$ROOT/Cargo.toml.bak"

# npm/package.json
sed -i.bak -E "s/\"version\": \"[0-9]+\.[0-9]+\.[0-9]+\"/\"version\": \"${BARE}\"/" "$ROOT/npm/package.json"
rm "$ROOT/npm/package.json.bak"

# Update Cargo.lock
(cd "$ROOT" && cargo check -q 2>/dev/null || true)

git -C "$ROOT" add Cargo.toml Cargo.lock npm/package.json
git -C "$ROOT" commit -m "chore: release ${TAG}"
git -C "$ROOT" tag "$TAG"
git -C "$ROOT" push origin main
git -C "$ROOT" push origin "$TAG"

echo "Released $TAG"
