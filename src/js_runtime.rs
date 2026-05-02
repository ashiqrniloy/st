use std::{
    cell::RefCell,
    fs, io,
    path::{Path, PathBuf},
    rc::Rc,
    thread,
};

use deno_core::{AsyncRefCell, OpState, RcRef, op2};
use deno_error::JsErrorBox;
use tokio::sync::mpsc as tokio_mpsc;

use crate::events::{EditorCommand, EditorEvent, RenderCommand};

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum JsRuntimeCommand {
    EditorCommand(EditorCommand),
    BindKey { chord: String, command_id: String },
}

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

#[op2]
pub fn op_send_editor_command(
    state: &mut OpState,
    #[serde] command: EditorCommand,
) -> Result<(), JsErrorBox> {
    let tx = state.borrow::<tokio_mpsc::UnboundedSender<JsRuntimeCommand>>();
    tx.send(JsRuntimeCommand::EditorCommand(command))
        .map_err(|_| JsErrorBox::generic("Editor command channel closed"))
}

#[op2(fast)]
pub fn op_bind_keybinding(
    state: &mut OpState,
    #[string] chord: String,
    #[string] command_id: String,
) -> Result<(), JsErrorBox> {
    let tx = state.borrow::<tokio_mpsc::UnboundedSender<JsRuntimeCommand>>();
    tx.send(JsRuntimeCommand::BindKey { chord, command_id })
        .map_err(|_| JsErrorBox::generic("Editor command channel closed"))
}

deno_core::extension!(
    js_runtime_extension,
    ops = [
        op_recv_editor_event,
        op_send_render_command,
        op_send_editor_command,
        op_bind_keybinding
    ],
);

pub fn runtime_dir() -> PathBuf {
    PathBuf::from("runtime")
}

fn runtime_file(path: &str) -> PathBuf {
    runtime_dir().join(path)
}

fn load_runtime_script(path: &Path) -> io::Result<String> {
    fs::read_to_string(path)
}

fn execute_script_file(js_runtime: &mut deno_core::JsRuntime, path: &Path) -> Result<(), String> {
    let code = load_runtime_script(path)
        .map_err(|err| format!("failed to load runtime file {}: {err}", path.display()))?;
    js_runtime
        .execute_script(path.to_string_lossy().to_string(), code)
        .map(|_| ())
        .map_err(|err| format!("runtime syntax/runtime error in {}: {err}", path.display()))
}

fn user_init_ts_path() -> PathBuf {
    crate::configuration::expand_config_path(crate::configuration::DEFAULT_INIT_JS, None)
}

fn execute_runtime_scripts_from_paths(
    js_runtime: &mut deno_core::JsRuntime,
    api_path: &Path,
    bootstrap_path: &Path,
) -> Result<(), String> {
    execute_runtime_scripts_from_paths_with_config(js_runtime, api_path, bootstrap_path, false)
}

fn execute_runtime_scripts_from_paths_with_config(
    js_runtime: &mut deno_core::JsRuntime,
    api_path: &Path,
    bootstrap_path: &Path,
    load_user_config: bool,
) -> Result<(), String> {
    let api_code = load_runtime_script(api_path)
        .map_err(|err| format!("failed to load runtime file {}: {err}", api_path.display()))?;
    let bootstrap_code = load_runtime_script(bootstrap_path).map_err(|err| {
        format!(
            "failed to load runtime file {}: {err}",
            bootstrap_path.display()
        )
    })?;

    js_runtime
        .execute_script(api_path.to_string_lossy().to_string(), api_code)
        .map_err(|err| {
            format!(
                "runtime syntax/runtime error in {}: {err}",
                api_path.display()
            )
        })?;

    let user_init = user_init_ts_path();
    if load_user_config && user_init.exists() {
        execute_script_file(js_runtime, &user_init)?;
    }

    js_runtime
        .execute_script(bootstrap_path.to_string_lossy().to_string(), bootstrap_code)
        .map_err(|err| {
            format!(
                "runtime syntax/runtime error in {}: {err}",
                bootstrap_path.display()
            )
        })?;

    Ok(())
}

fn execute_runtime_scripts_from_disk(js_runtime: &mut deno_core::JsRuntime) -> Result<(), String> {
    let api_path = runtime_file("editor_api.js");
    let bootstrap_path = runtime_file("bootstrap.js");
    execute_runtime_scripts_from_paths_with_config(js_runtime, &api_path, &bootstrap_path, true)
}

pub fn spawn_js_runtime(
    editor_event_rx: tokio_mpsc::UnboundedReceiver<EditorEvent>,
    render_tx: tokio_mpsc::UnboundedSender<RenderCommand>,
    editor_command_tx: tokio_mpsc::UnboundedSender<JsRuntimeCommand>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("build JS runtime thread runtime");

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
                state.put(editor_command_tx);
            }

            if let Err(err) = execute_runtime_scripts_from_disk(&mut js_runtime) {
                eprintln!("JS runtime load failure (server remains active): {err}");
                return;
            }

            if let Err(e) = js_runtime
                .run_event_loop(deno_core::PollEventLoopOptions {
                    wait_for_inspector: false,
                })
                .await
            {
                eprintln!("Deno event loop error: {e:?}");
            }

            println!("Logic thread: Deno runtime stopped.");
        });
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_file(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("st-{name}-{}", std::process::id()))
    }

    #[test]
    fn load_runtime_script_reads_file() {
        let path = temp_file("js-runtime-read.js");
        fs::write(&path, "globalThis.x = 1;").expect("write temp runtime file");
        let loaded = load_runtime_script(&path).expect("load file");
        assert!(loaded.contains("globalThis.x = 1"));
        let _ = fs::remove_file(path);
    }

    #[test]
    fn load_runtime_script_reports_missing_file() {
        let path = temp_file("missing-runtime.js");
        let err = load_runtime_script(&path).expect_err("missing file should error");
        assert_eq!(err.kind(), io::ErrorKind::NotFound);
    }

    #[test]
    fn runtime_loading_from_temp_files_succeeds() {
        let api = temp_file("runtime-api.js");
        let bootstrap = temp_file("runtime-bootstrap.js");
        fs::write(&api, "globalThis.testApi = true;").expect("write api");
        fs::write(&bootstrap, "globalThis.testBoot = 1;").expect("write bootstrap");

        let mut runtime = deno_core::JsRuntime::new(deno_core::RuntimeOptions::default());
        execute_runtime_scripts_from_paths(&mut runtime, &api, &bootstrap)
            .expect("runtime scripts should load");

        let _ = fs::remove_file(api);
        let _ = fs::remove_file(bootstrap);
    }

    #[test]
    fn syntax_errors_are_reported_clearly() {
        let api = temp_file("runtime-api-bad.js");
        let bootstrap = temp_file("runtime-bootstrap-good.js");
        fs::write(&api, "function () {").expect("write bad api");
        fs::write(&bootstrap, "globalThis.ok = true;").expect("write bootstrap");

        let mut runtime = deno_core::JsRuntime::new(deno_core::RuntimeOptions::default());
        let err = execute_runtime_scripts_from_paths(&mut runtime, &api, &bootstrap)
            .expect_err("syntax error expected");
        assert!(err.contains("runtime syntax/runtime error"));
        assert!(err.contains("runtime-api-bad.js"));

        let _ = fs::remove_file(api);
        let _ = fs::remove_file(bootstrap);
    }

    #[test]
    fn checked_in_runtime_scripts_load_together() {
        let mut runtime = deno_core::JsRuntime::new(deno_core::RuntimeOptions::default());
        execute_runtime_scripts_from_paths(
            &mut runtime,
            &runtime_file("editor_api.js"),
            &runtime_file("bootstrap.js"),
        )
        .expect("checked-in runtime scripts should load without syntax/runtime errors");
    }
}
