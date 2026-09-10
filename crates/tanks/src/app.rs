//! Winit-застосунок: орбіта камери, завантаження GLB, бій 200 на 200.

use crate::catalog::Catalog;
use crate::game::{Game, Input, FIXED_DT};
use crate::renderer::Renderer;
use std::sync::Arc;
use web_time::Instant;
use winit::application::ApplicationHandler;
use winit::dpi::PhysicalSize;
use winit::event::{ElementState, MouseButton, MouseScrollDelta, TouchPhase, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop, EventLoopProxy};
use winit::keyboard::{Key, KeyCode, NamedKey, PhysicalKey};
use winit::window::Window;

struct Ready {
    renderer: Renderer,
    catalog: Catalog,
}

#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
enum AppEvent {
    Ready(Box<Ready>),
}

struct App {
    #[allow(dead_code)]
    proxy: EventLoopProxy<AppEvent>,
    window: Option<Arc<Window>>,
    #[cfg(target_arch = "wasm32")]
    canvas: Option<web_sys::HtmlCanvasElement>,
    renderer: Option<Renderer>,
    catalog: Option<Catalog>,
    game: Game,
    input: Input,
    last_pointer: Option<(f64, f64)>,
    last_time: Option<Instant>,
    acc: f32,
    keys_x: f32,
    keys_z: f32,
}

impl App {
    fn new(proxy: EventLoopProxy<AppEvent>) -> Self {
        Self {
            proxy,
            window: None,
            #[cfg(target_arch = "wasm32")]
            canvas: None,
            renderer: None,
            catalog: None,
            game: Game::new(),
            input: Input::default(),
            last_pointer: None,
            last_time: None,
            acc: 0.0,
            keys_x: 0.0,
            keys_z: 0.0,
        }
    }

    fn apply_ready(&mut self, mut ready: Ready) {
        ready.renderer.upload_catalog(&ready.catalog);
        self.game.start(&ready.catalog);
        self.renderer = Some(ready.renderer);
        self.catalog = Some(ready.catalog);
        if let Some(w) = &self.window {
            w.request_redraw();
        }
    }

    fn frame(&mut self) {
        self.sync_surface_size();

        let now = Instant::now();
        let last = self.last_time.replace(now);
        let dt = last
            .map(|l| (now - l).as_secs_f32())
            .unwrap_or(FIXED_DT)
            .min(0.25);
        self.acc += dt;
        self.game.fps = self.game.fps * 0.9 + (1.0 / dt.max(1.0 / 600.0)) * 0.1;

        self.input.pan_x = self.keys_x;
        self.input.pan_z = self.keys_z;

        let extra = crate::pending::take_add();
        if extra > 0 {
            self.game.add_tanks_per_team(extra as usize);
        }

        let mut steps: u32 = 0;
        while self.acc >= FIXED_DT && steps < 8 {
            let input = self.input;
            self.game.update(FIXED_DT, &input);
            self.input.orbit_dx = 0.0;
            self.input.orbit_dy = 0.0;
            self.input.zoom = 0.0;
            self.acc -= FIXED_DT;
            steps += 1;
        }
        for s in self.game.drain_sfx() {
            engine::audio::play(s);
        }
        if self.acc > FIXED_DT {
            self.acc = 0.0;
        }

        if let Some(r) = &mut self.renderer {
            let _ = r.render(&self.game);
        }
        if let Some(w) = &self.window {
            w.set_title(&format!("Tanks — {}", self.game.overlay_msg()));
        }
        #[cfg(target_arch = "wasm32")]
        self.sync_overlay();
        if let Some(w) = &self.window {
            w.request_redraw();
        }
    }

    #[cfg(target_arch = "wasm32")]
    fn sync_surface_size(&mut self) {
        let Some(canvas) = &self.canvas else {
            return;
        };
        let dpr = web_sys::window()
            .map(|w| w.device_pixel_ratio())
            .unwrap_or(1.0);
        let css_w = canvas.client_width().max(1) as f64;
        let css_h = canvas.client_height().max(1) as f64;
        let target_w = (css_w * dpr).round().max(1.0) as u32;
        let target_h = (css_h * dpr).round().max(1.0) as u32;
        let Some(renderer) = &mut self.renderer else {
            return;
        };
        let (cfg_w, cfg_h) = renderer.size();
        if (cfg_w, cfg_h) != (target_w, target_h) {
            renderer.resize(target_w, target_h);
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn sync_surface_size(&mut self) {}

    #[cfg(target_arch = "wasm32")]
    fn sync_overlay(&self) {
        let Some(doc) = web_sys::window().and_then(|w| w.document()) else {
            return;
        };
        if let Some(el) = doc.get_element_by_id("overlay-msg") {
            el.set_text_content(Some(&self.game.overlay_msg()));
        }
    }
}

async fn boot_renderer(window: Arc<Window>) -> Ready {
    #[cfg(target_arch = "wasm32")]
    let surface_target = {
        use wasm_bindgen::JsCast;
        let canvas = web_sys::window()
            .expect("no global window")
            .document()
            .expect("no document")
            .get_element_by_id("canvas")
            .expect("no #canvas")
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .expect("element is not a canvas");
        wgpu::SurfaceTarget::Canvas(canvas)
    };

    #[cfg(not(target_arch = "wasm32"))]
    let surface_target = window.clone();

    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let surface = instance
        .create_surface(surface_target)
        .expect("Failed to create surface");
    let size = window.inner_size();
    let renderer = Renderer::new(instance, surface, size.width.max(1), size.height.max(1))
        .await
        .expect("wgpu init failed");
    let catalog = Catalog::load().await;
    Ready { renderer, catalog }
}

impl ApplicationHandler<AppEvent> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let attributes = {
            #[allow(unused_mut)]
            let mut attrs = Window::default_attributes()
        .with_title("Tanks — 200 vs 200")
                .with_resizable(true);

            #[cfg(not(target_arch = "wasm32"))]
            {
                attrs = attrs.with_inner_size(PhysicalSize::new(1280, 720));
            }

            #[cfg(target_arch = "wasm32")]
            {
                use wasm_bindgen::JsCast;
                use winit::platform::web::WindowAttributesExtWebSys;
                let canvas: web_sys::HtmlCanvasElement = web_sys::window()
                    .expect("no global window")
                    .document()
                    .expect("no document")
                    .get_element_by_id("canvas")
                    .expect("no #canvas")
                    .dyn_into()
                    .expect("#canvas is not a canvas");
                attrs = attrs.with_canvas(Some(canvas.clone()));
                self.canvas = Some(canvas);
            }

            attrs
        };

        let window = Arc::new(
            event_loop
                .create_window(attributes)
                .expect("Failed to create window"),
        );
        self.window = Some(window.clone());

        #[cfg(not(target_arch = "wasm32"))]
        {
            let ready = pollster::block_on(boot_renderer(window.clone()));
            self.apply_ready(ready);
        }

        #[cfg(target_arch = "wasm32")]
        {
            let proxy = self.proxy.clone();
            wasm_bindgen_futures::spawn_local(async move {
                let ready = boot_renderer(window).await;
                let _ = proxy.send_event(AppEvent::Ready(Box::new(ready)));
            });
        }
    }

    fn user_event(&mut self, _event_loop: &ActiveEventLoop, event: AppEvent) {
        match event {
            AppEvent::Ready(ready) => self.apply_ready(*ready),
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),

            WindowEvent::Resized(size) => {
                let size = PhysicalSize::new(size.width.max(1), size.height.max(1));
                if let Some(r) = &mut self.renderer {
                    r.resize(size.width, size.height);
                }
            }

            WindowEvent::CursorMoved { position, .. } => {
                if let Some((lx, ly)) = self.last_pointer {
                    if self.input.pointer_down {
                        self.input.orbit_dx += (position.x - lx) as f32;
                        self.input.orbit_dy += (position.y - ly) as f32;
                    }
                }
                self.last_pointer = Some((position.x, position.y));
            }

            WindowEvent::MouseInput { state, button, .. } => {
                if button == MouseButton::Left || button == MouseButton::Right {
                    self.input.pointer_down = state == ElementState::Pressed;
                    if self.input.pointer_down {
                        engine::audio::resume();
                    }
                    if !self.input.pointer_down {
                        self.last_pointer = None;
                    }
                }
            }

            WindowEvent::MouseWheel { delta, .. } => {
                let y = match delta {
                    MouseScrollDelta::LineDelta(_, y) => y,
                    MouseScrollDelta::PixelDelta(p) => (p.y as f32) * 0.05,
                };
                self.input.zoom -= y * 2.4;
            }

            WindowEvent::Touch(touch) => match touch.phase {
                TouchPhase::Started | TouchPhase::Moved => {
                    self.input.pointer_down = true;
                    if let Some((lx, ly)) = self.last_pointer {
                        self.input.orbit_dx += (touch.location.x - lx) as f32;
                        self.input.orbit_dy += (touch.location.y - ly) as f32;
                    }
                    self.last_pointer = Some((touch.location.x, touch.location.y));
                }
                _ => {
                    self.input.pointer_down = false;
                    self.last_pointer = None;
                }
            },

            WindowEvent::KeyboardInput { event, .. } => {
                let pressed = event.state == ElementState::Pressed;
                if pressed
                    && !event.repeat
                    && matches!(
                        event.physical_key,
                        PhysicalKey::Code(KeyCode::Digit1 | KeyCode::Numpad1)
                    )
                {
                    #[cfg(not(target_arch = "wasm32"))]
                    self.game.add_tanks_per_team(10);
                }
                match &event.logical_key {
                    Key::Named(NamedKey::Escape) if pressed => event_loop.exit(),
                    Key::Named(NamedKey::ArrowUp) | Key::Named(NamedKey::ArrowDown) => {
                        self.keys_z = if !pressed {
                            0.0
                        } else if matches!(event.logical_key, Key::Named(NamedKey::ArrowUp)) {
                            1.0
                        } else {
                            -1.0
                        };
                    }
                    Key::Named(NamedKey::ArrowLeft) | Key::Named(NamedKey::ArrowRight) => {
                        self.keys_x = if !pressed {
                            0.0
                        } else if matches!(event.logical_key, Key::Named(NamedKey::ArrowRight)) {
                            1.0
                        } else {
                            -1.0
                        };
                    }
                    Key::Character(c) => match c.to_lowercase().as_str() {
                        "w" => self.keys_z = if pressed { 1.0 } else { 0.0 },
                        "s" => self.keys_z = if pressed { -1.0 } else { 0.0 },
                        "a" => self.keys_x = if pressed { -1.0 } else { 0.0 },
                        "d" => self.keys_x = if pressed { 1.0 } else { 0.0 },
                        "r" if pressed => {
                            if let Some(cat) = &self.catalog {
                                self.game.restart(cat);
                            }
                        }
                        _ => {}
                    },
                    _ => {}
                }
            }

            WindowEvent::RedrawRequested => {
                if self.renderer.is_some() {
                    self.frame();
                }
            }

            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(w) = &self.window {
            if self.renderer.is_some() {
                w.request_redraw();
            }
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn run() {
    let event_loop = EventLoop::with_user_event().build().expect("event loop");
    let proxy = event_loop.create_proxy();
    let mut app = App::new(proxy);
    event_loop.run_app(&mut app).expect("event loop failed");
}

#[cfg(target_arch = "wasm32")]
pub fn run_web() {
    console_error_panic_hook::set_once();
    let event_loop = EventLoop::with_user_event().build().expect("event loop");
    let proxy = event_loop.create_proxy();
    let app = App::new(proxy);
    use winit::platform::web::EventLoopExtWebSys;
    event_loop.spawn_app(app);
}
