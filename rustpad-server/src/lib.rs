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

#[cfg(test)]
mod tests {
    use log::info;
    use operational_transform::OperationSeq;
    use serde_json::{json, Value};

    use super::*;

    #[tokio::test]
    async fn test_single_operation() {
        pretty_env_logger::try_init().ok();
        let filter = backend();

        let resp = warp::test::request().path("/text").reply(&filter).await;
        assert_eq!(resp.status(), 200);
        assert_eq!(resp.body(), "");

        let mut client = warp::test::ws()
            .path("/socket")
            .handshake(filter.clone())
            .await
            .expect("handshake");
        let msg = client.recv().await.unwrap();
        let msg = msg.to_str().unwrap();
        assert_eq!(msg, r#"{"Identity":0}"#);

        let mut operation = OperationSeq::default();
        operation.insert("hello");
        let serialized = format!(
            r#"{{"Edit": {{"revision": 0, "operation": {}}}}}"#,
            serde_json::to_string(&operation).unwrap(),
        );
        info!("sending ClientMsg {}", serialized);
        client.send_text(serialized).await;

        let msg = client.recv().await.unwrap();
        let msg = msg.to_str().unwrap();
        let msg: Value = serde_json::from_str(&msg).unwrap();
        assert_eq!(
            msg,
            json!({
                "History": {
                    "start": 0,
                    "operations": [
                        { "id": 0, "operation": ["hello"] }
                    ]
                }
            })
        );

        let resp = warp::test::request().path("/text").reply(&filter).await;
        assert_eq!(resp.status(), 200);
        assert_eq!(resp.body(), "hello");
    }

    #[tokio::test]
    async fn test_invalid_operation() {
        pretty_env_logger::try_init().ok();
        let filter = backend();

        let mut client = warp::test::ws()
            .path("/socket")
            .handshake(filter.clone())
            .await
            .expect("handshake");
        let msg = client.recv().await.unwrap();
        let msg = msg.to_str().unwrap();
        assert_eq!(msg, r#"{"Identity":0}"#);

        let mut operation = OperationSeq::default();
        operation.insert("hello");
        let serialized = format!(
            r#"{{"Edit": {{"revision": 1, "operation": {}}}}}"#,
            serde_json::to_string(&operation).unwrap(),
        );
        info!("sending ClientMsg {}", serialized);
        client.send_text(serialized).await;

        client.recv_closed().await.expect("socket should be closed");
    }
}
