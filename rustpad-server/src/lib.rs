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
    use std::time::Duration;

    use log::info;
    use operational_transform::OperationSeq;
    use serde_json::{json, Value};
    use tokio::time;
    use warp::test::WsClient;

    use super::*;

    /// A test WebSocket client that sends and receives JSON messages
    struct JsonSocket(WsClient);

    impl JsonSocket {
        async fn send(&mut self, msg: &Value) {
            self.0.send_text(msg.to_string()).await
        }

        async fn recv(&mut self) -> Value {
            let msg = self.0.recv().await.expect("recv failure");
            let msg = msg.to_str().expect("non-string message");
            serde_json::from_str(&msg).expect("non-json message")
        }

        async fn recv_closed(&mut self) {
            self.0.recv_closed().await.unwrap()
        }
    }

    /// Connect a new test client WebSocket
    async fn connect(filter: &BoxedFilter<(impl Reply + 'static,)>) -> JsonSocket {
        let client = warp::test::ws()
            .path("/socket")
            .handshake(filter.clone())
            .await
            .expect("handshake failed");
        JsonSocket(client)
    }

    /// Check the text route
    async fn expect_text(filter: &BoxedFilter<(impl Reply + 'static,)>, text: &str) {
        let resp = warp::test::request().path("/text").reply(filter).await;
        assert_eq!(resp.status(), 200);
        assert_eq!(resp.body(), text);
    }

    #[tokio::test]
    async fn test_single_operation() {
        pretty_env_logger::try_init().ok();
        let filter = backend();

        expect_text(&filter, "").await;

        let mut client = connect(&filter).await;
        let msg = client.recv().await;
        assert_eq!(msg, json!({ "Identity": 0 }));

        let mut operation = OperationSeq::default();
        operation.insert("hello");
        let msg = json!({
            "Edit": {
                "revision": 0,
                "operation": operation
            }
        });
        info!("sending ClientMsg {}", msg);
        client.send(&msg).await;

        let msg = client.recv().await;
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

        expect_text(&filter, "hello").await;
    }

    #[tokio::test]
    async fn test_invalid_operation() {
        pretty_env_logger::try_init().ok();
        let filter = backend();

        let resp = warp::test::request().path("/text").reply(&filter).await;
        assert_eq!(resp.status(), 200);
        assert_eq!(resp.body(), "");

        let mut client = connect(&filter).await;
        let msg = client.recv().await;
        assert_eq!(msg, json!({ "Identity": 0 }));

        let mut operation = OperationSeq::default();
        operation.insert("hello");
        let msg = json!({
            "Edit": {
                "revision": 1,
                "operation": operation
            }
        });
        info!("sending ClientMsg {}", msg);
        client.send(&msg).await;

        client.recv_closed().await;
    }

    #[tokio::test]
    async fn test_concurrent_transform() {
        pretty_env_logger::try_init().ok();
        let filter = backend();

        // Connect the first client
        let mut client = connect(&filter).await;
        let msg = client.recv().await;
        assert_eq!(msg, json!({ "Identity": 0 }));

        // Insert the first operation
        let mut operation = OperationSeq::default();
        operation.insert("hello");
        let msg = json!({
            "Edit": {
                "revision": 0,
                "operation": operation
            }
        });
        info!("sending ClientMsg {}", msg);
        client.send(&msg).await;

        let msg = client.recv().await;
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

        // Insert the second operation
        let mut operation = OperationSeq::default();
        operation.retain(2);
        operation.delete(1);
        operation.insert("n");
        operation.retain(2);
        let msg = json!({
            "Edit": {
                "revision": 1,
                "operation": operation
            }
        });
        info!("sending ClientMsg {}", msg);
        client.send(&msg).await;

        let msg = client.recv().await;
        assert_eq!(
            msg,
            json!({
                "History": {
                    "start": 1,
                    "operations": [
                        { "id": 0, "operation": [2, "n", -1, 2] }
                    ]
                }
            })
        );
        expect_text(&filter, "henlo").await;

        // Connect the second client
        let mut client2 = connect(&filter).await;
        let msg = client2.recv().await;
        assert_eq!(msg, json!({ "Identity": 1 }));

        // Insert a concurrent operation before seeing the existing history
        time::sleep(Duration::from_millis(50)).await;
        let mut operation = OperationSeq::default();
        operation.insert("~rust~");
        let msg = json!({
            "Edit": {
                "revision": 0,
                "operation": operation
            }
        });
        info!("sending ClientMsg {}", msg);
        client2.send(&msg).await;

        // Receive the existing history
        let msg = client2.recv().await;
        assert_eq!(
            msg,
            json!({
                "History": {
                    "start": 0,
                    "operations": [
                        { "id": 0, "operation": ["hello"] },
                        { "id": 0, "operation": [2, "n", -1, 2] }
                    ]
                }
            })
        );

        // Expect to receive a transformed operation
        let transformed_op = json!({
            "History": {
                "start": 2,
                "operations": [
                    { "id": 1, "operation": ["~rust~", 5] },
                ]
            }
        });

        // ... in the first client
        let msg = client.recv().await;
        assert_eq!(msg, transformed_op);

        // ... and in the second client
        let msg = client2.recv().await;
        assert_eq!(msg, transformed_op);
    }
}
