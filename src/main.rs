use std::num::NonZeroU32;
use std::rc::Rc;
use std::thread;
use serde::{Deserialize, Serialize};
use deno_core::{op2, OpState, AsyncRefCell, RcRef};
use deno_error::JsErrorBox;
use tokio::sync::mpsc as tokio_mpsc;
use winit::{
    application::ApplicationHandler,
    event::{ElementState, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Window, WindowId},
    keyboard::Key,
};
#[cfg(feature = "force-x11")]
use winit::platform::x11::EventLoopBuilderExtX11;

// Message definitions
#[derive(Debug, Serialize, Deserialize)]
pub enum UiEvent {
    KeyPress(char),
}

#[derive(Debug, Serialize, Deserialize)]
pub enum RenderCommand {
    DrawRect { x: f32, y: f32, w: f32, h: f32 },
}

// OP 1: Async Receiver
#[op2]
#[serde]
pub async fn op_recv_ui_event(state: Rc<std::cell::RefCell<OpState>>) -> Result<UiEvent, JsErrorBox> {
    let rx_cell = {
        let state_ref = state.borrow();
        state_ref.borrow::<Rc<AsyncRefCell<tokio_mpsc::UnboundedReceiver<UiEvent>>>>().clone()
    };
    let mut rx = RcRef::map(&rx_cell, |r| r).borrow_mut().await;
    rx.recv().await.ok_or_else(|| JsErrorBox::generic("UI channel closed"))
}

// OP 2: Synchronous sender
#[op2]
pub fn op_send_render_command(
    state: &mut OpState,
    #[serde] command: RenderCommand,
) -> Result<(), JsErrorBox> {
    let tx = state.borrow::<tokio_mpsc::UnboundedSender<RenderCommand>>();
    tx.send(command).map_err(|_| JsErrorBox::generic("Render channel closed"))
}

deno_core::extension!(
    editor_core,
    ops = [op_recv_ui_event, op_send_render_command],
);

// ---- winit App ---------------------------------------------------------------

struct App {
    window: Option<Rc<Window>>,
    // softbuffer context + surface for software rendering
    sb_context: Option<softbuffer::Context<Rc<Window>>>,
    sb_surface: Option<softbuffer::Surface<Rc<Window>, Rc<Window>>>,

    ui_tx:     tokio_mpsc::UnboundedSender<UiEvent>,
    ui_rx:     tokio_mpsc::UnboundedReceiver<RenderCommand>,
}

impl App {
    fn new(ui_tx: tokio_mpsc::UnboundedSender<UiEvent>,
           ui_rx: tokio_mpsc::UnboundedReceiver<RenderCommand>) -> Self {
        Self { window: None, sb_context: None, sb_surface: None, ui_tx, ui_rx }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let attrs = Window::default_attributes()
            .with_title("st")
            .with_inner_size(winit::dpi::LogicalSize::new(800.0_f64, 600.0_f64));
        let window = Rc::new(event_loop.create_window(attrs).unwrap());

        // Create softbuffer context and surface tied to this window
        let ctx = softbuffer::Context::new(window.clone()).unwrap();
        let surface = softbuffer::Surface::new(&ctx, window.clone()).unwrap();

        self.sb_context = Some(ctx);
        self.sb_surface = Some(surface);
        self.window = Some(window);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, window_id: WindowId, event: WindowEvent) {
        let Some(window) = self.window.as_ref() else { return };
        if window_id != window.id() { return; }

        match event {
            WindowEvent::CloseRequested => {
                println!("Closing st...");
                event_loop.exit();
            }

            WindowEvent::RedrawRequested => {
                let Some(surface) = self.sb_surface.as_mut() else { return };
                let size = window.inner_size();
                let w = size.width;
                let h = size.height;
                if w == 0 || h == 0 { return; }

                surface.resize(
                    NonZeroU32::new(w).unwrap(),
                    NonZeroU32::new(h).unwrap(),
                ).unwrap();

                let mut buf = surface.buffer_mut().unwrap();
                // Fill with a dark grey background (0x00RRGGBB)
                buf.fill(0x00_1e_1e_2e);
                buf.present().unwrap();
            }

            WindowEvent::KeyboardInput { event: key_event, .. } => {
                if key_event.state == ElementState::Pressed {
                    if let Key::Character(ch) = &key_event.logical_key {
                        if let Some(c) = ch.chars().next() {
                            println!("UI thread: key '{}'", c);
                            let _ = self.ui_tx.send(UiEvent::KeyPress(c));
                        }
                    }
                }
            }

            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        while let Ok(command) = self.ui_rx.try_recv() {
            println!("UI thread: render command from JS: {:?}", command);
            if let Some(window) = self.window.as_ref() {
                window.request_redraw();
            }
        }
    }
}

// ---- main --------------------------------------------------------------------

fn main() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();

    let (ui_tx, ui_rx_deno) = tokio_mpsc::unbounded_channel::<UiEvent>();
    let (logic_tx, ui_rx)   = tokio_mpsc::unbounded_channel::<RenderCommand>();

    // ---- Deno / Tokio thread -------------------------------------------------
    thread::spawn(move || {
        runtime.block_on(async {
            println!("Logic thread: Initializing Deno...");

            let mut js_runtime = deno_core::JsRuntime::new(deno_core::RuntimeOptions {
                extensions: vec![editor_core::init()],
                ..Default::default()
            });

            {
                let op_state = js_runtime.op_state();
                let mut state = op_state.borrow_mut();
                state.put(Rc::new(AsyncRefCell::new(ui_rx_deno)));
                state.put(logic_tx);
            }

            let js_code = r#"
                const { core } = Deno;

                async function mainLoop() {
                    while (true) {
                        const event = await core.ops.op_recv_ui_event();
                        core.print(`JS received: ${JSON.stringify(event)}\n`);

                        core.ops.op_send_render_command({
                            DrawRect: { x: 0.0, y: 0.0, w: 800.0, h: 600.0 }
                        });
                    }
                }

                mainLoop();
            "#;

            if let Err(e) = js_runtime.execute_script("<init>", js_code) {
                eprintln!("Deno execute_script error: {:?}", e);
                return;
            }

            loop {
                match js_runtime.run_event_loop(deno_core::PollEventLoopOptions {
                    wait_for_inspector: false,
                }).await {
                    Ok(()) => {
                        // In this embedding setup, run_event_loop can resolve even though
                        // JS is effectively waiting for the next UI event. Keep driving it.
                        tokio::task::yield_now().await;
                    }
                    Err(e) => {
                        let msg = e.to_string();
                        eprintln!("Deno event loop error: {:?}", e);
                        if msg.contains("UI channel closed") {
                            break;
                        }
                        tokio::task::yield_now().await;
                    }
                }
            }
        });
    });

    // ---- winit / UI thread (main) --------------------------------------------
    println!("Initializing Window...");

    let mut event_loop_builder = EventLoop::builder();
    #[cfg(feature = "force-x11")]
    event_loop_builder.with_x11();
    let event_loop = event_loop_builder.build().unwrap_or_else(|e| {
        eprintln!("ERROR: Could not create window event loop: {e}");
        std::process::exit(1);
    });
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = App::new(ui_tx, ui_rx);
    if let Err(e) = event_loop.run_app(&mut app) {
        eprintln!("winit event loop error: {:?}", e);
        std::process::exit(1);
    }
}
