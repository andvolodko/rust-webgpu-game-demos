# Build WASM demos into dist/ for local preview or GitHub Pages.
$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent $PSScriptRoot
Set-Location $Root

if (-not (Get-Command wasm-pack -ErrorAction SilentlyContinue)) {
  Write-Error "wasm-pack is required: cargo install wasm-pack"
}

rustup target add wasm32-unknown-unknown | Out-Null

Write-Host "==> Crystal Arkanoid"
wasm-pack build crates/arkanoid --target web --release --out-dir ../../arkanoid/pkg
Write-Host "==> Tanks"
wasm-pack build crates/tanks --target web --release --out-dir ../../tanks/pkg

Write-Host "==> Assemble dist/"
if (Test-Path dist) { Remove-Item -Recurse -Force dist }
New-Item -ItemType Directory -Force -Path dist/arkanoid, dist/tanks | Out-Null
Copy-Item index.html dist/
Copy-Item arkanoid/index.html dist/arkanoid/
Copy-Item -Recurse arkanoid/pkg dist/arkanoid/pkg
Copy-Item tanks/index.html dist/tanks/
Copy-Item -Recurse tanks/pkg dist/tanks/pkg
if (Test-Path assets) { Copy-Item -Recurse assets dist/assets }
Remove-Item -ErrorAction SilentlyContinue dist/arkanoid/pkg/.gitignore, dist/tanks/pkg/.gitignore
New-Item -ItemType File -Force -Path dist/.nojekyll | Out-Null

Write-Host "Done. Preview with: python serve.py"
Write-Host "GitHub Pages artifact root: dist/"
