//! Server backend for the Rustpad collaborative text editor.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use std::{sync::Arc, time::Duration};

use dashmap::DashMap;
use log::info;
use rustpad::Rustpad;
use tokio::time::{self, Instant};
use warp::{filters::BoxedFilter, ws::Ws, Filter, Reply};

mod rustpad;

/// An entry stored in the global server map.
///
/// Each entry corresponds to a single document. This is garbage collected by a
/// background task after one day of inactivity, to avoid server memory usage
/// growing without bound.
struct Document {
    last_accessed: Instant,
    rustpad: Arc<Rustpad>,
}

impl Default for Document {
    fn default() -> Self {
        Self {
            last_accessed: Instant::now(),
            rustpad: Default::default(),
        }
    }
}

/// A combined filter handling all server routes.
pub fn server() -> BoxedFilter<(impl Reply,)> {
    warp::path("api").and(backend()).or(frontend()).boxed()
}

/// Construct routes for static files from React.
fn frontend() -> BoxedFilter<(impl Reply,)> {
    warp::fs::dir("build")
        .or(warp::get().and(warp::fs::file("build/index.html")))
        .boxed()
}

/// Construct backend routes, including WebSocket handlers.
fn backend() -> BoxedFilter<(impl Reply,)> {
    let state: Arc<DashMap<String, Document>> = Default::default();
    tokio::spawn(cleaner(Arc::clone(&state)));

    let state_filter = warp::any().map(move || Arc::clone(&state));

    let socket = warp::path("socket")
        .and(warp::path::param())
        .and(warp::path::end())
        .and(warp::ws())
        .and(state_filter.clone())
        .map(
            |id: String, ws: Ws, state: Arc<DashMap<String, Document>>| {
                let mut entry = state.entry(id).or_default();
                let value = entry.value_mut();
                value.last_accessed = Instant::now();
                let rustpad = Arc::clone(&value.rustpad);
                ws.on_upgrade(|socket| async move { rustpad.on_connection(socket).await })
            },
        );

    let text = warp::path("text")
        .and(warp::path::param())
        .and(warp::path::end())
        .and(state_filter.clone())
        .map(|id: String, state: Arc<DashMap<String, Document>>| {
            state
                .get(&id)
                .map(|value| value.rustpad.text())
                .unwrap_or_default()
        });

    socket.or(text).boxed()
}

const HOUR: Duration = Duration::from_secs(3600);
const DAY: Duration = Duration::from_secs(24 * 3600);

// Reclaims memory for documents after a day of inactivity.
async fn cleaner(state: Arc<DashMap<String, Document>>) {
    loop {
        time::sleep(HOUR).await;
        let mut keys = Vec::new();
        for entry in &*state {
            if entry.last_accessed.elapsed() > DAY {
                keys.push(entry.key().clone());
            }
        }
        info!("cleaner removing keys: {:?}", keys);
        for key in keys {
            state.remove(&key);
        }
    }
}
