//! Asynchronous systems logic for Rustpad.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use anyhow::{bail, Context, Result};
use futures::prelude::*;
use log::{info, warn};
use operational_transform::OperationSeq;
use parking_lot::{RwLock, RwLockUpgradableReadGuard};
use serde::{Deserialize, Serialize};
use tokio::{sync::Notify, time};
use warp::ws::{Message, WebSocket};

/// The main object for a collaborative session.
#[derive(Default)]
pub struct Rustpad {
    state: RwLock<State>,
    count: AtomicU64,
    notify: Notify,
}

/// Shared state involving multiple users, protected by a lock.
#[derive(Default)]
struct State {
    operations: Vec<UserOperation>,
    text: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct UserOperation {
    id: u64,
    operation: OperationSeq,
}

/// A message received from the client over WebSocket.
#[derive(Clone, Debug, Serialize, Deserialize)]
enum ClientMsg {
    /// Represents a sequence of local edits from the user.
    Edit {
        revision: usize,
        operation: OperationSeq,
    },
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
}

impl From<ServerMsg> for Message {
    fn from(msg: ServerMsg) -> Self {
        let serialized = serde_json::to_string(&msg).expect("failed serialize");
        Message::text(serialized)
    }
}

impl Rustpad {
    /// Construct a new, empty Rustpad object.
    pub fn new() -> Self {
        Default::default()
    }

    /// Handle a connection from a WebSocket.
    pub async fn on_connection(&self, socket: WebSocket) {
        let id = self.count.fetch_add(1, Ordering::Relaxed);
        info!("connection! id = {}", id);
        if let Err(e) = self.handle_connection(id, socket).await {
            warn!("connection terminated early: {}", e);
        }
        info!("disconnection, id = {}", id);
    }

    /// Returns a snapshot of the latest text.
    pub fn text(&self) -> String {
        let state = self.state.read();
        state.text.clone()
    }

    /// Returns the current revision.
    pub fn revision(&self) -> usize {
        let state = self.state.read();
        state.operations.len()
    }

    async fn handle_connection(&self, id: u64, mut socket: WebSocket) -> Result<()> {
        socket.send(ServerMsg::Identity(id).into()).await?;

        let mut revision: usize = 0;

        loop {
            if self.revision() > revision {
                revision = self.send_history(revision, &mut socket).await?
            }

            let sleep = time::sleep(Duration::from_millis(500));
            tokio::pin!(sleep);
            tokio::select! {
                _ = &mut sleep => {}
                _ = self.notify.notified() => {}
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
        }
        Ok(())
    }

    fn apply_edit(&self, id: u64, revision: usize, mut operation: OperationSeq) -> Result<()> {
        let state = self.state.upgradable_read();
        let len = state.operations.len();
        if revision > len {
            bail!("got revision {}, but current is {}", revision, len);
        }
        for history_op in &state.operations[revision..] {
            operation = operation.transform(&history_op.operation)?.0;
        }
        let new_text = operation.apply(&state.text)?;
        let mut state = RwLockUpgradableReadGuard::upgrade(state);
        state.operations.push(UserOperation { id, operation });
        state.text = new_text;
        Ok(())
    }
}
