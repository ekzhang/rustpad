//! Eventually consistent server-side logic for Rustpad.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;

use anyhow::{bail, Context, Result};
use futures::prelude::*;
use log::{info, warn};
use operational_transform::OperationSeq;
use parking_lot::{Mutex, RwLock, RwLockUpgradableReadGuard};
use serde::{Deserialize, Serialize};
use tokio::sync::{broadcast, watch, Notify};
use warp::ws::{Message, WebSocket};

use crate::runner::{RunStage, Runner, Runtime};
use crate::{database::PersistedDocument, ot::transform_index};

/// The main object representing a collaborative session.
pub struct Rustpad {
    /// State modified by critical sections of the code.
    state: RwLock<State>,
    /// Incremented to obtain unique user IDs.
    count: AtomicU64,
    /// Used to notify clients of new text operations.
    notify: Notify,
    /// Used to inform all clients of metadata updates.
    update: broadcast::Sender<ServerMsg>,
    /// Set to true when the document is destroyed.
    killed: AtomicBool,
    /// Code runner access, if code execution is enabled.
    runner: Option<Runner>,
    /// State of code runs, shared with the task running the code.
    run: Arc<Mutex<RunState>>,
}

/// Shared state involving multiple users, protected by a lock.
#[derive(Default)]
struct State {
    operations: Vec<UserOperation>,
    text: String,
    language: Option<String>,
    users: HashMap<u64, UserInfo>,
    cursors: HashMap<u64, CursorData>,
}

/// Code run state for one document, kept in memory only.
#[derive(Default)]
struct RunState {
    /// User ID of the run in progress.
    running: Option<u64>,
    /// The last `RunFinished` or `RunFailed` message.
    last: Option<ServerMsg>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct UserOperation {
    id: u64,
    operation: OperationSeq,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct UserInfo {
    name: String,
    hue: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct CursorData {
    cursors: Vec<u32>,
    selections: Vec<(u32, u32)>,
}

/// A message received from the client over WebSocket.
#[derive(Clone, Debug, Serialize, Deserialize)]
enum ClientMsg {
    /// Represents a sequence of local edits from the user.
    Edit {
        revision: usize,
        operation: OperationSeq,
    },
    /// Sets the language of the editor.
    SetLanguage(String),
    /// Sets the user's current information.
    ClientInfo(UserInfo),
    /// Sets the user's cursor and selection positions.
    CursorData(CursorData),
    /// Runs the current document with its language's runtime.
    Run,
}

/// A message sent to the client over WebSocket.
#[derive(Clone, Debug, Serialize, Deserialize)]
enum ServerMsg {
    /// Informs the client of their unique socket ID.
    Identity(u64),
    /// Broadcasts text operations to all clients.
    History {
        start: usize,
        operations: Vec<UserOperation>,
    },
    /// Broadcasts the current language, last writer wins.
    Language(String),
    /// Broadcasts a user's information, or `None` on disconnect.
    UserInfo { id: u64, info: Option<UserInfo> },
    /// Broadcasts a user's cursor position.
    UserCursor { id: u64, data: CursorData },
    /// Lists the languages this server can run.
    Runtimes(Vec<Runtime>),
    /// Broadcasts that a user started a run.
    RunStarted { id: u64 },
    /// Broadcasts the result of a finished run.
    RunFinished {
        id: u64,
        language: String,
        runtime: String,
        version: String,
        compile: Option<Box<RunStage>>,
        run: Option<Box<RunStage>>,
        duration_ms: u64,
    },
    /// Broadcasts that a run could not complete.
    RunFailed { id: u64, error: String },
}

impl From<ServerMsg> for Message {
    fn from(msg: ServerMsg) -> Self {
        let serialized = serde_json::to_string(&msg).expect("failed serialize");
        Message::text(serialized)
    }
}

impl Default for Rustpad {
    fn default() -> Self {
        let (tx, _) = broadcast::channel(16);
        Self {
            state: Default::default(),
            count: Default::default(),
            notify: Default::default(),
            update: tx,
            killed: AtomicBool::new(false),
            runner: None,
            run: Default::default(),
        }
    }
}

impl From<PersistedDocument> for Rustpad {
    fn from(document: PersistedDocument) -> Self {
        let mut operation = OperationSeq::default();
        operation.insert(&document.text);

        let rustpad = Self::default();
        {
            let mut state = rustpad.state.write();
            state.text = document.text;
            state.language = document.language;
            state.operations.push(UserOperation {
                id: u64::MAX,
                operation,
            })
        }
        rustpad
    }
}

/// Largest document that can be run, in bytes. go-judge accepts more, but the
/// server keeps the request and the shared result small.
const MAX_RUN_CODE_BYTES: usize = 64 * 1024;

impl Rustpad {
    /// Handle a connection from a WebSocket.
    pub async fn on_connection(&self, socket: WebSocket) {
        let id = self.count.fetch_add(1, Ordering::Relaxed);
        info!("connection! id = {}", id);
        if let Err(e) = self.handle_connection(id, socket).await {
            warn!("connection terminated early: {}", e);
        }
        info!("disconnection, id = {}", id);
        self.state.write().users.remove(&id);
        self.state.write().cursors.remove(&id);
        self.update
            .send(ServerMsg::UserInfo { id, info: None })
            .ok();
    }

    /// Returns a snapshot of the latest text.
    pub fn text(&self) -> String {
        let state = self.state.read();
        state.text.clone()
    }

    /// Returns a snapshot of the current document for persistence.
    pub fn snapshot(&self) -> PersistedDocument {
        let state = self.state.read();
        PersistedDocument {
            text: state.text.clone(),
            language: state.language.clone(),
        }
    }

    /// Returns the current revision.
    pub fn revision(&self) -> usize {
        let state = self.state.read();
        state.operations.len()
    }

    /// Kill this object immediately, dropping all current connections.
    pub fn kill(&self) {
        self.killed.store(true, Ordering::Relaxed);
        self.notify.notify_waiters();
    }

    /// Returns if this Rustpad object has been killed.
    pub fn killed(&self) -> bool {
        self.killed.load(Ordering::Relaxed)
    }

    /// Enables code execution for this document.
    pub(crate) fn with_runner(mut self, runner: Option<Runner>) -> Self {
        self.runner = runner;
        self
    }

    async fn handle_connection(&self, id: u64, mut socket: WebSocket) -> Result<()> {
        let mut update_rx = self.update.subscribe();

        let mut runtimes_rx = self.runner.as_ref().map(|r| r.runtimes.clone());

        let mut revision: usize = self
            .send_initial(id, &mut socket, runtimes_rx.as_mut())
            .await?;

        loop {
            // In order to avoid the "lost wakeup" problem, we first request a
            // notification, **then** check the current state for new revisions.
            // This is the same approach that `tokio::sync::watch` takes.
            let notified = self.notify.notified();
            if self.killed() {
                break;
            }
            if self.revision() > revision {
                revision = self.send_history(revision, &mut socket).await?
            }

            tokio::select! {
                _ = notified => {}
                update = update_rx.recv() => {
                    socket.send(update?.into()).await?;
                }
                Some(runtimes) = next_runtimes(&mut runtimes_rx) => {
                    socket.send(ServerMsg::Runtimes(runtimes).into()).await?;
                }
                result = socket.next() => {
                    match result {
                        None => break,
                        Some(message) => {
                            self.handle_message(id, message?).await?;
                        }
                    }
                }
            }
        }

        Ok(())
    }

    async fn send_initial(
        &self,
        id: u64,
        socket: &mut WebSocket,
        runtimes_rx: Option<&mut watch::Receiver<Vec<Runtime>>>,
    ) -> Result<usize> {
        socket.send(ServerMsg::Identity(id).into()).await?;
        let mut messages = Vec::new();
        let revision = {
            let state = self.state.read();
            if !state.operations.is_empty() {
                messages.push(ServerMsg::History {
                    start: 0,
                    operations: state.operations.clone(),
                });
            }
            if let Some(language) = &state.language {
                messages.push(ServerMsg::Language(language.clone()));
            }
            for (&id, info) in &state.users {
                messages.push(ServerMsg::UserInfo {
                    id,
                    info: Some(info.clone()),
                });
            }
            for (&id, data) in &state.cursors {
                messages.push(ServerMsg::UserCursor {
                    id,
                    data: data.clone(),
                });
            }
            if let Some(rx) = runtimes_rx {
                messages.push(ServerMsg::Runtimes(rx.borrow_and_update().clone()));
            }
            let run = self.run.lock();
            if let Some(id) = run.running {
                messages.push(ServerMsg::RunStarted { id });
            } else if let Some(last) = &run.last {
                messages.push(last.clone());
            }
            drop(run);
            state.operations.len()
        };
        for msg in messages {
            socket.send(msg.into()).await?;
        }
        Ok(revision)
    }

    async fn send_history(&self, start: usize, socket: &mut WebSocket) -> Result<usize> {
        let operations = {
            let state = self.state.read();
            let len = state.operations.len();
            if start < len {
                state.operations[start..].to_owned()
            } else {
                Vec::new()
            }
        };
        let num_ops = operations.len();
        if num_ops > 0 {
            let msg = ServerMsg::History { start, operations };
            socket.send(msg.into()).await?;
        }
        Ok(start + num_ops)
    }

    async fn handle_message(&self, id: u64, message: Message) -> Result<()> {
        let msg: ClientMsg = match message.to_str() {
            Ok(text) => serde_json::from_str(text).context("failed to deserialize message")?,
            Err(()) => return Ok(()), // Ignore non-text messages
        };
        match msg {
            ClientMsg::Edit {
                revision,
                operation,
            } => {
                self.apply_edit(id, revision, operation)
                    .context("invalid edit operation")?;
                self.notify.notify_waiters();
            }
            ClientMsg::SetLanguage(language) => {
                self.state.write().language = Some(language.clone());
                self.update.send(ServerMsg::Language(language)).ok();
            }
            ClientMsg::ClientInfo(info) => {
                self.state.write().users.insert(id, info.clone());
                let msg = ServerMsg::UserInfo {
                    id,
                    info: Some(info),
                };
                self.update.send(msg).ok();
            }
            ClientMsg::CursorData(data) => {
                self.state.write().cursors.insert(id, data.clone());
                let msg = ServerMsg::UserCursor { id, data };
                self.update.send(msg).ok();
            }
            ClientMsg::Run => self.start_run(id),
        }
        Ok(())
    }

    /// Starts running the document for user `id`, unless a run is in progress.
    fn start_run(&self, id: u64) {
        let Some(runner) = &self.runner else {
            warn!("run requested by id = {}, but code execution is off", id);
            return;
        };
        let (text, language) = {
            let state = self.state.read();
            let language = state.language.clone();
            (
                state.text.clone(),
                language.unwrap_or_else(|| "plaintext".into()),
            )
        };
        let runtime = runner
            .runtimes
            .borrow()
            .iter()
            .find(|r| r.language == language)
            .cloned();

        let mut run = self.run.lock();
        if run.running.is_some() {
            info!("run requested by id = {} while busy, ignoring", id);
            return;
        }
        let Some(runtime) = runtime else {
            let msg = ServerMsg::RunFailed {
                id,
                error: format!("No runtime for {language}"),
            };
            run.last = Some(msg.clone());
            self.update.send(msg).ok();
            return;
        };
        if text.len() > MAX_RUN_CODE_BYTES {
            let msg = ServerMsg::RunFailed {
                id,
                error: "Code too large to run (64 KiB limit)".into(),
            };
            run.last = Some(msg.clone());
            self.update.send(msg).ok();
            return;
        }
        run.running = Some(id);
        self.update.send(ServerMsg::RunStarted { id }).ok();
        drop(run);

        info!(
            "run: id = {}, runtime = {} {}",
            id, runtime.runtime, runtime.version
        );
        let client = runner.client.clone();
        let run_state = Arc::clone(&self.run);
        let update = self.update.clone();
        tokio::spawn(async move {
            let started = Instant::now();
            let result = client.execute(&runtime.language, &text).await;
            let msg = match result {
                Ok(output) => ServerMsg::RunFinished {
                    id,
                    language: runtime.language,
                    runtime: runtime.runtime,
                    version: runtime.version,
                    compile: output.compile.map(Box::new),
                    run: output.run.map(Box::new),
                    duration_ms: started.elapsed().as_millis() as u64,
                },
                Err(e) => ServerMsg::RunFailed {
                    id,
                    error: e.to_string(),
                },
            };
            let mut run = run_state.lock();
            run.running = None;
            run.last = Some(msg.clone());
            update.send(msg).ok();
        });
    }

    fn apply_edit(&self, id: u64, revision: usize, mut operation: OperationSeq) -> Result<()> {
        info!(
            "edit: id = {}, revision = {}, base_len = {}, target_len = {}",
            id,
            revision,
            operation.base_len(),
            operation.target_len()
        );
        let state = self.state.upgradable_read();
        let len = state.operations.len();
        if revision > len {
            bail!("got revision {}, but current is {}", revision, len);
        }
        for history_op in &state.operations[revision..] {
            operation = operation.transform(&history_op.operation)?.0;
        }
        if operation.target_len() > 256 * 1024 {
            bail!(
                "target length {} is greater than 256 KiB maximum",
                operation.target_len()
            );
        }
        let new_text = operation.apply(&state.text)?;
        let mut state = RwLockUpgradableReadGuard::upgrade(state);
        for (_, data) in state.cursors.iter_mut() {
            for cursor in data.cursors.iter_mut() {
                *cursor = transform_index(&operation, *cursor);
            }
            for (start, end) in data.selections.iter_mut() {
                *start = transform_index(&operation, *start);
                *end = transform_index(&operation, *end);
            }
        }
        state.operations.push(UserOperation { id, operation });
        state.text = new_text;
        Ok(())
    }
}

/// Waits for the runtime list to change, or forever if code execution is off.
async fn next_runtimes(rx: &mut Option<watch::Receiver<Vec<Runtime>>>) -> Option<Vec<Runtime>> {
    match rx {
        Some(rx) => {
            rx.changed().await.ok()?;
            Some(rx.borrow_and_update().clone())
        }
        None => future::pending().await,
    }
}
