mod editor_core;
mod rendering;

use tokio::sync::mpsc as tokio_mpsc;

use crate::{
    editor_core::{RenderCommand, UiEvent, spawn_js_runtime},
    rendering::{UiChannels, run_ui_with_gpui},
};

fn main() {
    let (ui_tx, ui_rx_deno) = tokio_mpsc::unbounded_channel::<UiEvent>();
    let (logic_tx, render_rx) = tokio_mpsc::unbounded_channel::<RenderCommand>();

    let js_thread = spawn_js_runtime(ui_rx_deno, logic_tx);

    println!("Initializing GPUI window...");

    let ui_result = run_ui_with_gpui(UiChannels { ui_tx, render_rx });

    if let Err(err) = ui_result {
        eprintln!("UI error: {err}");
        std::process::exit(1);
    }

    if let Err(err) = js_thread.join() {
        eprintln!("Logic thread panicked: {:?}", err);
        std::process::exit(1);
    }
}
