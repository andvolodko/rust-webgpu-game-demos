#!/usr/bin/env bash
# Build WASM demos into dist/ for local preview or GitHub Pages.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

if ! command -v wasm-pack >/dev/null 2>&1; then
  echo "wasm-pack is required: cargo install wasm-pack" >&2
  exit 1
fi

rustup target add wasm32-unknown-unknown >/dev/null

echo "==> Crystal Arkanoid"
wasm-pack build crates/arkanoid --target web --release --out-dir ../../arkanoid/pkg

echo "==> Tanks"
wasm-pack build crates/tanks --target web --release --out-dir ../../tanks/pkg

echo "==> Assemble dist/"
rm -rf dist
mkdir -p dist/arkanoid dist/tanks
cp index.html dist/
cp arkanoid/index.html dist/arkanoid/
cp -R arkanoid/pkg dist/arkanoid/pkg
cp tanks/index.html dist/tanks/
cp -R tanks/pkg dist/tanks/pkg
if [ -d assets ]; then
  cp -R assets dist/assets
fi
rm -f dist/arkanoid/pkg/.gitignore dist/tanks/pkg/.gitignore
touch dist/.nojekyll

echo "Done. Preview: python serve.py  (or serve dist/ after copying)"
echo "GitHub Pages artifact root: dist/"
