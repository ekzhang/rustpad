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
        .and(rustpad)
        .map(|ws: Ws, rustpad: Arc<Rustpad>| {
            ws.on_upgrade(move |socket| async move { rustpad.on_connection(socket).await })
        });

    socket.boxed()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_single_message() {
        let filter = backend();
        let mut client = warp::test::ws()
            .path("/socket")
            .handshake(filter)
            .await
            .expect("handshake");
        client.send_text("hello world").await;
        let msg = client.recv().await.expect("recv");
        let msg = msg.to_str().expect("string");
        assert_eq!(msg, "[[0,\"hello world\"]]");
    }
}
