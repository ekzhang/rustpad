//! Server routes for Rustpad

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use futures::prelude::*;
use log::{error, info};
use tokio::{
    sync::{Notify, RwLock},
    time,
};
use warp::{
    filters::BoxedFilter,
    ws::{Message, WebSocket, Ws},
    Filter, Reply,
};

/// Construct a set of routes for the server
pub fn routes() -> BoxedFilter<(impl Reply,)> {
    let rustpad = Arc::new(Rustpad::default());
    let rustpad = warp::any().map(move || Arc::clone(&rustpad));

    let socket = warp::path("socket")
        .and(warp::path::end())
        .and(warp::ws())
        .and(rustpad)
        .map(|ws: Ws, rustpad: Arc<Rustpad>| {
            ws.on_upgrade(move |socket| async move { rustpad.on_connection(socket).await })
        });

    socket.boxed()
}

/// The main object for a collaborative session
#[derive(Default)]
struct Rustpad {
    state: RwLock<State>,
    count: AtomicU64,
    notify: Notify,
}

/// Shared state involving multiple users, protected by a lock
#[derive(Default)]
struct State {
    messages: Vec<(u64, String)>,
}

impl Rustpad {
    async fn on_connection(&self, mut socket: WebSocket) {
        let id = self.count.fetch_add(1, Ordering::Relaxed);
        info!("connection! id = {}", id);

        let mut revision: usize = 0;

        loop {
            if self.num_messages().await > revision {
                match self.send_messages(revision, &mut socket).await {
                    Ok(new_revision) => revision = new_revision,
                    Err(e) => {
                        error!("websocket error: {}", e);
                        break;
                    }
                }
            }

            let sleep = time::sleep(Duration::from_millis(500));
            tokio::pin!(sleep);
            tokio::select! {
                _ = &mut sleep => {}
                _ = self.notify.notified() => {}
                result = socket.next() => {
                    match result {
                        None => break,
                        Some(Ok(message)) => {
                            self.handle_message(id, message).await
                        }
                        Some(Err(e)) => {
                            error!("websocket error: {}", e);
                            break;
                        }
                    }
                }
            }
        }

        info!("disconnection, id = {}", id);
    }

    async fn num_messages(&self) -> usize {
        let state = self.state.read().await;
        state.messages.len()
    }

    async fn send_messages(
        &self,
        revision: usize,
        socket: &mut WebSocket,
    ) -> Result<usize, warp::Error> {
        let state = self.state.read().await;
        let len = state.messages.len();
        if revision < len {
            let messages = serde_json::to_string(&state.messages[revision..])
                .expect("serde serialization failed for messages");
            drop(state);
            socket.send(Message::text(&messages)).await?;
            Ok(len)
        } else {
            Ok(revision)
        }
    }

    async fn handle_message(&self, id: u64, message: Message) {
        let text = match message.to_str() {
            Ok(text) => String::from(text),
            Err(()) => return, // Ignore non-text messages
        };

        let mut state = self.state.write().await;
        state.messages.push((id, text));
        self.notify.notify_waiters();
    }
}
