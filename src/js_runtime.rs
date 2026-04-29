use std::{cell::RefCell, rc::Rc, thread};

use deno_core::{AsyncRefCell, OpState, RcRef, op2};
use deno_error::JsErrorBox;
use tokio::sync::mpsc as tokio_mpsc;

use crate::events::{EditorEvent, RenderCommand};

#[op2]
#[serde]
pub async fn op_recv_editor_event(
    state: Rc<RefCell<OpState>>,
) -> Result<Option<EditorEvent>, JsErrorBox> {
    let rx_cell = {
        let state_ref = state.borrow();
        state_ref
            .borrow::<Rc<AsyncRefCell<tokio_mpsc::UnboundedReceiver<EditorEvent>>>>()
            .clone()
    };
    let mut rx = RcRef::map(&rx_cell, |r| r).borrow_mut().await;
    Ok(rx.recv().await)
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
    js_runtime_extension,
    ops = [op_recv_editor_event, op_send_render_command],
);

pub fn spawn_js_runtime(
    editor_event_rx: tokio_mpsc::UnboundedReceiver<EditorEvent>,
    render_tx: tokio_mpsc::UnboundedSender<RenderCommand>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();

        runtime.block_on(async move {
            println!("Logic thread: Initializing Deno...");

            let mut js_runtime = deno_core::JsRuntime::new(deno_core::RuntimeOptions {
                extensions: vec![js_runtime_extension::init()],
                ..Default::default()
            });

            {
                let op_state = js_runtime.op_state();
                let mut state = op_state.borrow_mut();
                state.put(Rc::new(AsyncRefCell::new(editor_event_rx)));
                state.put(render_tx);
            }

            let js_code = r#"
                const { core } = Deno;

                async function mainLoop() {
                    while (true) {
                        const event = await core.ops.op_recv_editor_event();
                        if (event === null) {
                            core.print("JS loop: editor event channel closed, stopping runtime loop.\n");
                            break;
                        }

                        core.print(`JS received: ${JSON.stringify(event)}\n`);

                        core.ops.op_send_render_command({
                            DrawRect: { x: 0.0, y: 0.0, w: 100.0, h: 100.0, color: 0xff00ff }
                        });
                    }
                }

                mainLoop().catch((err) => {
                    core.print(`JS mainLoop error: ${err?.stack ?? err}\n`);
                });
            "#;

            if let Err(e) = js_runtime.execute_script("<init>", js_code) {
                eprintln!("Deno execute_script error: {:?}", e);
                return;
            }

            if let Err(e) = js_runtime
                .run_event_loop(deno_core::PollEventLoopOptions {
                    wait_for_inspector: false,
                })
                .await
            {
                eprintln!("Deno event loop error: {:?}", e);
            }

            println!("Logic thread: Deno runtime stopped.");
        });
    })
}
