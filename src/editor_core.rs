use std::{cell::RefCell, rc::Rc, thread};

use deno_core::{AsyncRefCell, OpState, RcRef, op2};
use deno_error::JsErrorBox;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc as tokio_mpsc;

#[derive(Debug, Serialize, Deserialize)]
pub enum UiEvent {
    KeyPress(char),
}

#[derive(Debug, Serialize, Deserialize)]
pub enum RenderCommand {
    DrawRect {
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        color: u32,
    },
}

#[op2]
#[serde]
pub async fn op_recv_ui_event(state: Rc<RefCell<OpState>>) -> Result<UiEvent, JsErrorBox> {
    let rx_cell = {
        let state_ref = state.borrow();
        state_ref
            .borrow::<Rc<AsyncRefCell<tokio_mpsc::UnboundedReceiver<UiEvent>>>>()
            .clone()
    };
    let mut rx = RcRef::map(&rx_cell, |r| r).borrow_mut().await;
    rx.recv()
        .await
        .ok_or_else(|| JsErrorBox::generic("UI channel closed"))
}

#[op2]
pub fn op_send_render_command(
    state: &mut OpState,
    #[serde] command: RenderCommand,
) -> Result<(), JsErrorBox> {
    let tx = state.borrow::<tokio_mpsc::UnboundedSender<RenderCommand>>();
    tx.send(command)
        .map_err(|_| JsErrorBox::generic("Render channel closed"))
}

deno_core::extension!(
    editor_core,
    ops = [op_recv_ui_event, op_send_render_command],
);

pub fn spawn_js_runtime(
    ui_rx_deno: tokio_mpsc::UnboundedReceiver<UiEvent>,
    logic_tx: tokio_mpsc::UnboundedSender<RenderCommand>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();

        runtime.block_on(async move {
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
                            DrawRect: { x: 0.0, y: 0.0, w: 100.0, h: 100.0, color: 0xff00ff }
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
                match js_runtime
                    .run_event_loop(deno_core::PollEventLoopOptions {
                        wait_for_inspector: false,
                    })
                    .await
                {
                    Ok(()) => {
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
    })
}
