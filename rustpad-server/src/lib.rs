//! Server backend for the Rustpad collaborative text editor.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use std::sync::Arc;

use dashmap::DashMap;
use rustpad::Rustpad;
use warp::{filters::BoxedFilter, ws::Ws, Filter, Reply};

mod rustpad;

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
    let rustpad_map: Arc<DashMap<String, Arc<Rustpad>>> = Default::default();
    let rustpad_map = warp::any().map(move || Arc::clone(&rustpad_map));

    let socket = warp::path("socket")
        .and(warp::path::param())
        .and(warp::path::end())
        .and(warp::ws())
        .and(rustpad_map.clone())
        .map(
            |id: String, ws: Ws, rustpad_map: Arc<DashMap<String, Arc<Rustpad>>>| {
                let rustpad = rustpad_map.entry(id).or_default();
                let rustpad = Arc::clone(rustpad.value());
                ws.on_upgrade(move |socket| async move { rustpad.on_connection(socket).await })
            },
        );

    let text = warp::path("text")
        .and(warp::path::param())
        .and(warp::path::end())
        .and(rustpad_map.clone())
        .map(
            |id: String, rustpad_map: Arc<DashMap<String, Arc<Rustpad>>>| {
                rustpad_map
                    .get(&id)
                    .map(|rustpad| rustpad.text())
                    .unwrap_or_default()
            },
        );

    socket.or(text).boxed()
}
