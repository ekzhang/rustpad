//! Server backend for the Rustpad collaborative text editor.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use std::sync::Arc;
use std::time::{Duration, SystemTime};

use dashmap::DashMap;
use log::info;
use serde::Serialize;
use tokio::time::{self, Instant};
use warp::{filters::BoxedFilter, ws::Ws, Filter, Reply};

use rustpad::Rustpad;

mod ot;
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

/// Statistics about the server, returned from an API endpoint.
#[derive(Serialize)]
struct Stats {
    /// System time when the server started, in seconds since Unix epoch.
    start_time: u64,
    /// Number of documents currently tracked by the server.
    num_documents: usize,
}

/// Server configuration.
#[derive(Debug)]
pub struct ServerConfig {
    /// Number of days to clean up documents after inactivity.
    pub expiry_days: u32,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self { expiry_days: 1 }
    }
}

/// A combined filter handling all server routes.
pub fn server(config: ServerConfig) -> BoxedFilter<(impl Reply,)> {
    warp::path("api")
        .and(backend(config))
        .or(frontend())
        .boxed()
}

/// Construct routes for static files from React.
fn frontend() -> BoxedFilter<(impl Reply,)> {
    warp::fs::dir("build").boxed()
}

/// Construct backend routes, including WebSocket handlers.
fn backend(config: ServerConfig) -> BoxedFilter<(impl Reply,)> {
    let state: Arc<DashMap<String, Document>> = Default::default();
    tokio::spawn(cleaner(Arc::clone(&state), config.expiry_days));

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

    let start_time = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("SystemTime returned before UNIX_EPOCH")
        .as_secs();
    let stats = warp::path("stats")
        .and(warp::path::end())
        .and(state_filter.clone())
        .map(move |state: Arc<DashMap<String, Document>>| {
            let num_documents = state.len();
            warp::reply::json(&Stats {
                start_time,
                num_documents,
            })
        });

    socket.or(text).or(stats).boxed()
}

const HOUR: Duration = Duration::from_secs(3600);

// Reclaims memory for documents.
async fn cleaner(state: Arc<DashMap<String, Document>>, expiry_days: u32) {
    loop {
        time::sleep(HOUR).await;
        let mut keys = Vec::new();
        for entry in &*state {
            if entry.last_accessed.elapsed() > HOUR * 24 * expiry_days {
                keys.push(entry.key().clone());
            }
        }
        info!("cleaner removing keys: {:?}", keys);
        for key in keys {
            state.remove(&key);
        }
    }
}
