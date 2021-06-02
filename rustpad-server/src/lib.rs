//! Server backend for the Rustpad collaborative text editor

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use std::sync::Arc;

use rustpad::Rustpad;
use warp::{filters::BoxedFilter, ws::Ws, Filter, Reply};

mod rustpad;

/// A combined filter handling all server routes
pub fn server() -> BoxedFilter<(impl Reply,)> {
    warp::path("api").and(backend()).or(frontend()).boxed()
}

/// Construct routes for static files from React
fn frontend() -> BoxedFilter<(impl Reply,)> {
    warp::fs::dir("build")
        .or(warp::get().and(warp::fs::file("build/index.html")))
        .boxed()
}

/// Construct backend routes, including WebSocket handlers
fn backend() -> BoxedFilter<(impl Reply,)> {
    let rustpad = Arc::new(Rustpad::new());
    let rustpad = warp::any().map(move || Arc::clone(&rustpad));

    let socket = warp::path("socket")
        .and(warp::path::end())
        .and(warp::ws())
        .and(rustpad.clone())
        .map(|ws: Ws, rustpad: Arc<Rustpad>| {
            ws.on_upgrade(move |socket| async move { rustpad.on_connection(socket).await })
        });

    let text = warp::path("text")
        .and(warp::path::end())
        .and(rustpad.clone())
        .map(|rustpad: Arc<Rustpad>| rustpad.text());

    socket.or(text).boxed()
}
