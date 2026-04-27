use std::{process::Command, thread};
use std::rc::Rc;
use std::cell::RefCell;
use serde::{Deserialize, Serialize};
use deno_core::{op2, OpState, error::AnyError, error::custom_error};
use tokio::runtime;
use tokio::sync::Mutex;
use tokio::sync::mpsc;
use winit::window;
use winit::{
    event::{Event, WindowEvent, ElementState},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
    keyboard::Key,
};

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
#[op2(async)]
#[serde]
pub async fn op_recv_ui_event(state: Rc<RefCell<OpState>>) -> Result<UiEvent, AnyError> {
    // extract the channel receiver from Deno's global state
    let rx_mutex = {
        let state_ref = state.borrow();
        state_ref.borrow::<Rc<Mutex<mpsc::UnboundedReceiver<UiEvent>>>>().clone()
    };

    // lock the mutex and wait for the UI thread to send an event
    let mut rx = rx_mutex.lock().await;
    let event = rx.recv().await.ok_or_else(|| {
        custom_error("ChannelError", "UI Channel Closed") // Fixed "Channeld" typo
    })?;

    Ok(event)
}

// synchronous sender. Deno calls this to send a drawing command to UI thread
#[op2]
pub fn op_send_render_command(
    state: &mut OpState,
    #[serde] command: RenderCommand,
) -> Result<(), AnyError> {
    let tx = state.borrow::<mpsc::UnboundedSender<RenderCommand>>();
    
    tx.send(command).map_err(|_| {
        custom_error("ChannelError", "Logic channel closed")
    })?;

    Ok(())
}

deno_core::extension!(
    editor_core,
    ops = [op_recv_ui_event, op_send_render_command],
);

fn main() {
    // create threads for UI events
    let (ui_tx, mut logic_rx) = mpsc::unbounded_channel::<UiEvent>();
    let (logic_tx, mut ui_rx) = mpsc::unbounded_channel::<RenderCommand>();

    // background thread for Tokio & Deno
    thread::spawn(move || {
        let runtime = tokio::runtime::Runtime::new().unwrap();

        runtime.block_on(async {
            println!("Logic thread: Initializing Deno...");

            // initialize custom ops
            let mut js_runtime = deno_core::JsRuntime::new(deno_core::RuntimeOptions {
                extensions: vec![editor_core::init_ops_and_esm()],
                ..Default::default()
            });

            {
                // FIX: Was `op_start`, corrected to `op_state`
                let op_state = js_runtime.op_state(); 
                // FIX: Was `borrow.mut()`, corrected to `borrow_mut()`
                let mut state = op_state.borrow_mut(); 

                // wrap receiver into mutex as it is used across async
                state.put(Rc::new(Mutex::new(logic_rx)));

                state.put(logic_tx);
            }

            // evaluate TS/JS logic
            let js_code = r#"
                const { core } = Deno;

                // The main logic loop
                async function mainLoop() {
                    while (true) {
                        // Await input from Rust UI thread
                        const event = await core.ops.op_recv_ui_event();
                        // FIX: Was `${JSON.stringify(event_}`, corrected closing parenthesis
                        core.print(`JS Received: ${JSON.stringify(event)}\n`); 

                        // Future code for agentic logic

                        // Send rendering command back to Rust UI thread
                        core.ops.op_send_render_command({
                            DrawRect: { x:0.0, y: 0.0, w:800.0, h: 600.0 } 
                        });
                    }
                }

                mainLoop();
            "#;

            js_runtime.execute_script("<init>", js_code).unwrap();
            js_runtime.run_event_loop(Default::default()).await.unwrap();
        });
    });

    // main thread for UI
    println!("Initializing Window...");

    let event_loop = EventLoop::new().unwrap();
    let window = WindowBuilder::new()
        .with_title("st")
        .with_inner_size(winit::dpi::LogicalSize::new(800.0, 600.0))
        .build(&event_loop)
        .unwrap();

    // UI loop on the main thread. Code below this will never run
    event_loop.run(move |event, elwt| {
        elwt.set_control_flow(ControlFlow::Poll);

        match event {
            Event::WindowEvent { window_id, event } if window_id == window.id() => {
                match event {
                    WindowEvent::CloseRequested => {
                        println!("Closing st...");
                        elwt.exit();
                    }

                    // catch user pressing key
                    WindowEvent::KeyboardInput { event: key_event, .. } => {
                        if key_event.state == ElementState::Pressed {
                            if let Key::Character(ch) = &key_event.logical_key {
                                if let Some(c) = ch.chars().next() {
                                    println!("UI Thread: Caught physical key '{}'", c);
                                    let _ = ui_tx.send(UiEvent::KeyPress(c));
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
            Event::AboutToWait => {
                // check for pending draw commands
                while let Ok(command) = ui_rx.try_recv() {
                    println!("UI Thread: Received command from TS: {:?}", command);

                    // send request to GPU to be implemented
                    // for now request OS to draw
                    window.request_redraw();
                }
            }
            _ => {}
        }
    }).unwrap();

}