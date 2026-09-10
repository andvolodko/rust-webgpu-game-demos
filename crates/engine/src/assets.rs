//! Завантаження байтів з `assets/` (native FS або HTTP fetch у WASM).

/// Шукає файл відносно `assets/<relative>`.
/// Native: cwd, потім корінь репо від `CARGO_MANIFEST_DIR` викликаючого крейта.
/// WASM: `GET /assets/<relative>`.
pub async fn load_bytes(relative: &str) -> Result<Vec<u8>, String> {
    load_bytes_from("assets", relative).await
}

pub async fn load_bytes_from(root: &str, relative: &str) -> Result<Vec<u8>, String> {
    let relative = relative.trim_start_matches(['/', '\\']);

    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = root;
        load_bytes_native(root, relative)
    }

    #[cfg(target_arch = "wasm32")]
    {
        load_bytes_web(root, relative).await
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn load_bytes_native(root: &str, relative: &str) -> Result<Vec<u8>, String> {
    let rel = std::path::Path::new(root).join(relative);
    let mut candidates = vec![std::env::current_dir().ok().map(|p| p.join(&rel)).unwrap_or_else(|| rel.clone())];

    // crates/<name>/src → repo root
    let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    candidates.push(manifest.join("../../").join(&rel));
    candidates.push(manifest.join(&rel));

    for path in candidates {
        if let Ok(bytes) = std::fs::read(&path) {
            return Ok(bytes);
        }
    }
    Err(format!("asset not found: {}/{}", root, relative.replace('\\', "/")))
}

#[cfg(target_arch = "wasm32")]
async fn load_bytes_web(root: &str, relative: &str) -> Result<Vec<u8>, String> {
    use wasm_bindgen::JsCast;
    use wasm_bindgen_futures::JsFuture;
    use web_sys::Response;

    // Relative to the demo page (`/tanks/` or `/repo/tanks/`) so GitHub Pages works.
    let url = format!("../{}/{}", root.trim_matches('/'), relative.replace('\\', "/"));
    let window = web_sys::window().ok_or("no window")?;
    let resp_val = JsFuture::from(window.fetch_with_str(&url))
        .await
        .map_err(|e| format!("fetch {url}: {e:?}"))?;
    let resp: Response = resp_val
        .dyn_into()
        .map_err(|_| format!("fetch {url}: not a Response"))?;
    if !resp.ok() {
        return Err(format!("fetch {url}: HTTP {}", resp.status()));
    }
    let buf = JsFuture::from(resp.array_buffer().map_err(|e| format!("{e:?}"))?)
        .await
        .map_err(|e| format!("arrayBuffer {url}: {e:?}"))?;
    let arr = js_sys::Uint8Array::new(&buf);
    let mut bytes = vec![0u8; arr.length() as usize];
    arr.copy_to(&mut bytes);
    Ok(bytes)
}
