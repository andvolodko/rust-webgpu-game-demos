# Rust + WebGPU demos

Two games, one engine:

| | Crystal Arkanoid | Tanks |
|---|---|---|
| Run | `cargo run --release` | `cargo run -p tanks --release` |
| Web | `/arkanoid/` | `/tanks/` |

Needs **WebGPU** (Chrome / Edge 113+, Firefox, Safari 18+). Not WebGL.

## Play (native)

```sh
cargo run --release           # Arkanoid
cargo run -p tanks --release  # Tanks
```

**Arkanoid:** A/D or pointer — paddle · Space/tap — launch · X — practice floor · 1–5 — level · R — restart.  
Power-ups: expand, slow, pierce, extra life, **multiball (+3 even if you already have 3)**, **laser**.

**Tanks:** 200 GREEN vs 200 STEEL. WASD — pan · LMB drag — orbit · wheel — zoom · R — new battle · **1** — +10 tanks per team.

## Play (browser, local)

```sh
rustup target add wasm32-unknown-unknown
cargo install wasm-pack
powershell -File scripts/build-web.ps1   # or: bash scripts/build-web.sh
python serve.py
```

Then open http://localhost:8000  
From another device on the LAN use **https://IP:8443** (WebGPU requires HTTPS).

`python serve.py` serves the **repo root** (good for day-to-day). `dist/` is the copy GitHub Pages uses.

## Models (Tanks)

All in one folder — no `GREEN/` / `BW/` subdirs:

```
assets/tanks/models/tank_1_green.glb
assets/tanks/models/tank_1_bw.glb
assets/tanks/models/tree.glb … tree4.glb
```

FBX is not loaded. Convert to a self-contained `.glb` (see `assets/tanks/models/README.md`). Missing files fall back to boxes.

## GitHub Pages

**Advice:** commit `dist/` and let CI only *upload* it. A full `wasm-pack` on Actions works, but the first run pulls a Rust toolchain + wasm-pack (several minutes, extra minutes of billed time). For this repo the wasm is small; checking in `dist/` is simpler and Pages stays online even if Actions is broken.

After you change the web games or assets:

```sh
powershell -File scripts/build-web.ps1
git add dist
git commit -m "Rebuild GitHub Pages site"
```

Workflow: `.github/workflows/pages.yml`  
Repo setting: **Settings → Pages → Source: GitHub Actions**.

URL: `https://USER.github.io/REPO/` (asset paths are relative, so a project site works).

## Credits

- Tanks: [FREE stylized tank 3D model](https://mreliptik.itch.io/free-lowpoly-tank-3d-model) by [MrEliptik](https://mreliptik.itch.io) (green / black-and-white variants).
- Trees: [10+ Free Low Poly Trees Pack](https://crazydrpants.itch.io/free-low-poly-trees-pack) by [CrazyDrPants](https://crazydrpants.itch.io).

This project was built with [Cursor](https://cursor.com) and Grok 4.6.

## Audio

Sounds are synthesized in `crates/engine` (no wav/mp3). Browsers stay silent until the first click or key.

## Layout

```
crates/engine      shared math, GPU, GLB, tilt, SFX
crates/arkanoid    Crystal Arkanoid
crates/tanks       200 vs 200 battle
assets/tanks/models/
arkanoid/  tanks/  HTML shells; wasm-pack writes pkg/ (gitignored)
dist/              static site for GitHub Pages (committed)
scripts/build-web.ps1 | build-web.sh
```
