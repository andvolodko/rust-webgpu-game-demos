//! Winit-застосунок: вікно, ввід, фіксований timestep, рендер.

use crate::game::{Game, GameState, Input, FIXED_DT, WORLD_W};
use crate::gfx::Renderer;
use std::sync::Arc;
use web_time::Instant;
use winit::application::ApplicationHandler;
use winit::dpi::PhysicalSize;
use winit::event::{ElementState, MouseButton, TouchPhase, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop, EventLoopProxy};
use winit::keyboard::{Key, NamedKey};
use winit::window::Window;

#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
enum AppEvent {
    RendererReady(Box<Renderer>),
}

struct App {
    #[allow(dead_code)]
    proxy: EventLoopProxy<AppEvent>,
    window: Option<Arc<Window>>,
    #[cfg(target_arch = "wasm32")]
    canvas: Option<web_sys::HtmlCanvasElement>,
    renderer: Option<Renderer>,
    game: Game,
    input: Input,
    last_time: Option<Instant>,
    acc: f32,
    render_failures: u32,
}

impl App {
    fn new(proxy: EventLoopProxy<AppEvent>) -> Self {
        Self {
            proxy,
            window: None,
            #[cfg(target_arch = "wasm32")]
            canvas: None,
            renderer: None,
            game: Game::new(),
            input: Input::default(),
            last_time: None,
            acc: 0.0,
            render_failures: 0,
        }
    }

    fn frame(&mut self) {
        self.sync_surface_size();
        self.sample_tilt();

        let now = Instant::now();
        let last = self.last_time.replace(now);
        let dt = last
            .map(|l| (now - l).as_secs_f32())
            .unwrap_or(FIXED_DT)
            .min(0.25);
        self.acc += dt;
        self.game.fps = self.game.fps * 0.9 + (1.0 / dt.max(1.0 / 600.0)) * 0.1;

        let mut steps: u32 = 0;
        while self.acc >= FIXED_DT && steps < 8 {
            let input = self.input;
            self.game.update(FIXED_DT, &input);
            self.input.launch = false;
            self.acc -= FIXED_DT;
            steps += 1;
        }
        if self.acc > FIXED_DT {
            self.acc = 0.0;
        }

        if let Some(r) = &mut self.renderer {
            let shown = r.render(&mut self.game, dt);
            if !shown {
                self.render_failures = self.render_failures.saturating_add(1);
                #[cfg(target_arch = "wasm32")]
                if self.render_failures == 1
                    || self.render_failures == 60
                    || self.render_failures % 300 == 0
                {
                    web_sys::console::error_1(
                        &format!(
                            "render() did not present a frame ({} consecutive failures) — surface out of sync",
                            self.render_failures
                        )
                        .into(),
                    );
                }

                #[cfg(not(target_arch = "wasm32"))]
                if self.render_failures == 1 || self.render_failures % 300 == 0 {
                    eprintln!(
                        "render() failed to present a frame ({} consecutive failures) — surface out of sync",
                        self.render_failures
                    );
                }
            } else {
                self.render_failures = 0;
            }
        }
        #[cfg(target_arch = "wasm32")]
        self.sync_overlay();
        if let Some(w) = &self.window {
            w.request_redraw();
        }
    }

    fn sample_tilt(&mut self) {
        let (gamma, beta) = crate::tilt::get();
        self.input.tilt_x = (gamma / 28.0).clamp(-1.0, 1.0);
        self.input.tilt_y = ((beta - 50.0) / 40.0).clamp(-1.0, 1.0);
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
            el.set_text_content(Some(self.game.overlay_msg()));
        }
        if let Some(el) = doc.get_element_by_id("overlay-level") {
            let text = if self.game.immortal {
                format!("LEVEL {}  ·  PRACTICE", self.game.level)
            } else {
                format!("LEVEL {}", self.game.level)
            };
            el.set_text_content(Some(&text));
        }
        if let Some(el) = doc.get_element_by_id("hud-overlay") {
            let playing = matches!(self.game.state, GameState::Playing);
            let _ = el.set_attribute("data-playing", if playing { "1" } else { "0" });
        }
    }
}

fn screen_to_world_x(x: f64, surface_w: u32) -> f32 {
    (x / (surface_w.max(1) as f64) * WORLD_W as f64) as f32
}

impl ApplicationHandler<AppEvent> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let attributes = {
            #[allow(unused_mut)]
            let mut attrs = Window::default_attributes()
                .with_title("Crystal Arkanoid — Rust + wgpu")
                .with_resizable(true);

            #[cfg(not(target_arch = "wasm32"))]
            {
                attrs = attrs.with_inner_size(PhysicalSize::new(800, 600));
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
                    .expect("no #canvas element in index.html")
                    .dyn_into()
                    .expect("#canvas is not a canvas element");
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

        let init = {
            let window = window.clone();
            async move {
                #[cfg(target_arch = "wasm32")]
                let surface_target = {
                    use wasm_bindgen::JsCast;
                    let canvas = web_sys::window()
                        .expect("no global window")
                        .document()
                        .expect("no document")
                        .get_element_by_id("canvas")
                        .expect("no #canvas element")
                        .dyn_into::<web_sys::HtmlCanvasElement>()
                        .expect("element is not a canvas");
                    wgpu::SurfaceTarget::Canvas(canvas)
                };

                #[cfg(not(target_arch = "wasm32"))]
                let surface_target = window.clone();

                let instance =
                    wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
                let surface = instance
                    .create_surface(surface_target)
                    .expect("Failed to create surface");

                let size = window.inner_size();
                Renderer::new(instance, surface, size.width.max(1), size.height.max(1))
                    .await
                    .expect("wgpu init failed")
            }
        };

        #[cfg(not(target_arch = "wasm32"))]
        {
            let renderer = pollster::block_on(init);
            self.renderer = Some(renderer);
            window.request_redraw();
        }

        #[cfg(target_arch = "wasm32")]
        {
            let proxy = self.proxy.clone();
            wasm_bindgen_futures::spawn_local(async move {
                let renderer = init.await;
                let _ = proxy.send_event(AppEvent::RendererReady(Box::new(renderer)));
            });
        }
    }

    fn user_event(&mut self, _event_loop: &ActiveEventLoop, event: AppEvent) {
        match event {
            AppEvent::RendererReady(renderer) => {
                self.renderer = Some(*renderer);
                if let Some(w) = &self.window {
                    w.request_redraw();
                }
            }
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
                #[cfg(target_arch = "wasm32")]
                self.sync_surface_size();
            }

            WindowEvent::CursorMoved { position, .. } => {
                if let Some(w) = &self.window {
                    let size = w.inner_size();
                    self.input.pointer_x = Some(screen_to_world_x(position.x, size.width));
                }
            }

            WindowEvent::Touch(touch) => {
                if let Some(w) = &self.window {
                    let size = w.inner_size();
                    self.input.pointer_x = Some(screen_to_world_x(touch.location.x, size.width));
                }
                if touch.phase == TouchPhase::Started {
                    self.input.launch = true;
                }
            }

            WindowEvent::MouseInput { state, button, .. } => {
                if button == MouseButton::Left && state == ElementState::Pressed {
                    self.input.launch = true;
                }
            }

            WindowEvent::KeyboardInput { event, .. } => {
                if event.state == ElementState::Pressed {
                    match event.logical_key {
                        Key::Named(NamedKey::Space) => self.input.launch = true,
                        Key::Named(NamedKey::Escape) => {
                            if self.game.state == GameState::Playing {
                                self.game.state = GameState::Ready;
                            } else {
                                event_loop.exit();
                            }
                        }
                        Key::Character(c) => match c.to_lowercase().as_str() {
                            "a" => self.input.axis = -1.0,
                            "d" => self.input.axis = 1.0,
                            "r" => self.game.restart(),
                            "x" => self.game.toggle_immortal(),
                            "1" => self.game.goto_level(1),
                            "2" => self.game.goto_level(2),
                            "3" => self.game.goto_level(3),
                            "4" => self.game.goto_level(4),
                            "5" => self.game.goto_level(5),
                            _ => {}
                        },
                        _ => {}
                    }
                } else {
                    self.input.axis = 0.0;
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
