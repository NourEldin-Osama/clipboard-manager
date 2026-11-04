//! The llama-servers the app starts: one for the search model and one for
//! the reading model, each with its own model and port. The app stops them
//! when it quits.

use std::ffi::OsString;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::{Mutex, MutexGuard};
use std::time::Duration;

use anyhow::{Context, Result};

use crate::config::LaunchConfig;

/// What a llama-server the app starts is for.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Role {
    /// Makes the vectors meaning search compares: started with `--embeddings`.
    Search,
    /// Reads the text in images.
    Read,
}

impl Role {
    fn slot(self) -> usize {
        match self {
            Role::Search => 0,
            Role::Read => 1,
        }
    }
}

/// The servers started, by role; kept globally so they can be stopped on
/// exit even though the worker thread never unwinds.
static RUNNING: Mutex<[Option<Child>; 2]> = Mutex::new([None, None]);

fn running() -> MutexGuard<'static, [Option<Child>; 2]> {
    RUNNING.lock().unwrap_or_else(|e| e.into_inner())
}

/// Starts llama-server for `role`, after stopping the one started for it
/// before. It runs in the background, without a window.
pub fn start(role: Role, launch: &LaunchConfig) -> Result<()> {
    stop(role);
    let mut cmd = Command::new(&launch.llama_server);
    cmd.args(args(role, launch))
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    let child = cmd
        .spawn()
        .with_context(|| format!("starting {}", launch.llama_server.display()))?;
    running()[role.slot()] = Some(child);
    Ok(())
}

pub fn stop(role: Role) {
    if let Some(mut child) = running()[role.slot()].take() {
        let _ = child.kill();
        let _ = child.wait();
    }
}

/// Stops every llama-server the app started.
pub fn stop_all() {
    stop(Role::Search);
    stop(Role::Read);
}

/// How the llama-server started for `role` ended, if it has.
pub fn exited(role: Role) -> Result<Option<ExitStatus>> {
    match running()[role.slot()].as_mut() {
        Some(child) => Ok(child.try_wait()?),
        None => Ok(None),
    }
}

/// Whether the llama-server at `base_url` (ending in `/v1`) has loaded its
/// model. It doesn't answer at all while it downloads the model.
pub fn ready(base_url: &str) -> bool {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(2)))
        .http_status_as_error(false)
        .build()
        .into();
    let root = base_url
        .trim()
        .trim_end_matches('/')
        .trim_end_matches("/v1");
    agent
        .get(&format!("{root}/health"))
        .call()
        .is_ok_and(|r| r.status().is_success())
}

/// What llama-server is started with: the model from a file, or by its
/// Hugging Face name (`-hf`, which downloads it the first time);
/// `--embeddings` only for search; the address; the projector file; and the
/// user's own arguments.
fn args(role: Role, l: &LaunchConfig) -> Vec<OsString> {
    let mut args: Vec<OsString> = Vec::new();
    match l.hf.trim() {
        "" => args.extend(["-m".into(), l.gguf.clone().into()]),
        name => args.extend(["-hf".into(), name.into()]),
    }
    if role == Role::Search {
        args.push("--embeddings".into());
    }
    args.extend([
        "--host".into(),
        "127.0.0.1".into(),
        "--port".into(),
        l.port.to_string().into(),
    ]);
    if let Some(projector) = &l.mmproj {
        args.extend(["--mmproj".into(), projector.clone().into()]);
    }
    args.extend(l.extra_args.split_whitespace().map(OsString::from));
    args
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args_of(role: Role, l: &LaunchConfig) -> String {
        let args: Vec<String> = args(role, l)
            .into_iter()
            .map(|a| a.to_string_lossy().into_owned())
            .collect();
        args.join(" ")
    }

    #[test]
    fn a_search_model_gets_embeddings_and_a_reading_model_does_not() {
        let file = LaunchConfig {
            llama_server: "llama-server".into(),
            gguf: "embeddinggemma.gguf".into(),
            port: 8765,
            extra_args: "--pooling mean -c 2048".into(),
            ..Default::default()
        };
        assert_eq!(
            args_of(Role::Search, &file),
            "-m embeddinggemma.gguf --embeddings --host 127.0.0.1 --port 8765 --pooling mean -c 2048"
        );
        let by_name = LaunchConfig {
            hf: " ggml-org/GLM-OCR-GGUF ".into(),
            gguf: "left over.gguf".into(),
            mmproj: Some("mmproj.gguf".into()),
            port: 8766,
            extra_args: "-c 8192".into(),
            ..Default::default()
        };
        // The name wins over a file, and a reading model gets no --embeddings.
        assert_eq!(
            args_of(Role::Read, &by_name),
            "-hf ggml-org/GLM-OCR-GGUF --host 127.0.0.1 --port 8766 --mmproj mmproj.gguf -c 8192"
        );
    }
}
