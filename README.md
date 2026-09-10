# Crystal Arkanoid — Rust + wgpu

Полірований Arkanoid: скляні кристали, 3D-уламки, bloom, рівні, combo і power-upи.
Компілюється у WASM і працює в браузері через **WebGPU** (без WebGL2).

Партікли (~131k: 65 536 уламків + 65 536 іскор) симулюються compute-шейдером на GPU.
CPU лише емітить spawn.

## Керування
- **Мишка / палець / A-D** — рух платформи
- **Space / клік / тап** — запуск м'яча
- **Нахил телефону** — платформа + легкий зсув камери
- **X** — practice: м'яч відбивається від дна, програти не можна
- **ESC** — пауза
- **R** — рестарт
- **1–5** — стрибок на рівень

## Power-upи
Падають з розбитих цеглин (~28%):
- **Expand** (бірюза) — ширша платформа
- **Multiball** (золото) — до 3 м'ячів
- **Slow** (синій) — повільніші м'ячі
- **Pierce** (фіолет) — м'яч пробиває цеглини
- **Extra life** (рожевий)

Димчасте скло не б’ється, але на 4–5 рівнях завжди є прохід знизу. Combo росте, поки не відіб’єшся від платформи (макс ×8). 5 рівнів, 6 життів.

## Локальний запуск (native)
```sh
cargo run --release
```

## Збірка в браузер (WASM)
1. Встановити інструменти (один раз):
```sh
rustup target add wasm32-unknown-unknown
cargo install wasm-pack
```

2. Зібрати:
```sh
wasm-pack build --target web --release --out-dir pkg
```

3. Запустити сервер (слухає всі інтерфейси, не лише localhost):
```sh
python serve.py          # HTTP :8000 + HTTPS :8443 (якщо є openssl)
```

4. Відкрити:
- на цій машині: http://localhost:8000
- з телефону / іншого ПК в LAN: **https://IP:8443** (самопідписаний сертифікат — Allow / Advanced → Proceed). HTTP з LAN не є secure context, тож WebGPU там не стартує.

> Підтримка: Chrome / Edge 113+, Firefox з WebGPU, Safari 18+. WebGL2 fallback прибрано.

## Архітектура
- **src/lib.rs** — точки входу `run()` / `start()`
- **src/app.rs** — winit event loop, ввід, фіксований timestep
- **src/game/** — платформа, м'ячі, цеглини, рівні, combo, power-upи
- **src/particles.rs** — CPU-емітер (spawn у GPU storage)
- **src/gfx/** — рендерер, сцена, HUD, bloom, GPU compute-партікли
- **src/shaders/** — WGSL (кристалі, bloom, compute-симуляція)
- **src/math.rs** — Vec2/Vec3/Mat4, AABB, колізії
