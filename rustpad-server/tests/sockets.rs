use std::time::Duration;

use anyhow::{anyhow, Result};
use log::info;
use operational_transform::OperationSeq;
use rustpad_server::server;
use serde_json::{json, Value};
use tokio::time;
use warp::{filters::BoxedFilter, test::WsClient, Reply};

/// A test WebSocket client that sends and receives JSON messages
struct JsonSocket(WsClient);

impl JsonSocket {
    async fn send(&mut self, msg: &Value) {
        self.0.send_text(msg.to_string()).await
    }

    async fn recv(&mut self) -> Result<Value> {
        let msg = self.0.recv().await?;
        let msg = msg.to_str().map_err(|_| anyhow!("non-string message"))?;
        Ok(serde_json::from_str(&msg)?)
    }

    async fn recv_closed(&mut self) -> Result<()> {
        self.0.recv_closed().await.map_err(|e| e.into())
    }
}

/// Connect a new test client WebSocket
async fn connect(filter: &BoxedFilter<(impl Reply + 'static,)>) -> Result<JsonSocket> {
    let client = warp::test::ws()
        .path("/api/socket")
        .handshake(filter.clone())
        .await?;
    Ok(JsonSocket(client))
}

/// Check the text route
async fn expect_text(filter: &BoxedFilter<(impl Reply + 'static,)>, text: &str) {
    let resp = warp::test::request().path("/api/text").reply(filter).await;
    assert_eq!(resp.status(), 200);
    assert_eq!(resp.body(), text);
}

#[tokio::test]
async fn test_single_operation() -> Result<()> {
    pretty_env_logger::try_init().ok();
    let filter = server();

    expect_text(&filter, "").await;

    let mut client = connect(&filter).await?;
    let msg = client.recv().await?;
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

    let msg = client.recv().await?;
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
    Ok(())
}

#[tokio::test]
async fn test_invalid_operation() -> Result<()> {
    pretty_env_logger::try_init().ok();
    let filter = server();

    expect_text(&filter, "").await;

    let mut client = connect(&filter).await?;
    let msg = client.recv().await?;
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

    client.recv_closed().await?;
    Ok(())
}

#[tokio::test]
async fn test_concurrent_transform() -> Result<()> {
    pretty_env_logger::try_init().ok();
    let filter = server();

    // Connect the first client
    let mut client = connect(&filter).await?;
    let msg = client.recv().await?;
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

    let msg = client.recv().await?;
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

    let msg = client.recv().await?;
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
    let mut client2 = connect(&filter).await?;
    let msg = client2.recv().await?;
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
    let msg = client2.recv().await?;
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
    let msg = client.recv().await?;
    assert_eq!(msg, transformed_op);

    // ... and in the second client
    let msg = client2.recv().await?;
    assert_eq!(msg, transformed_op);

    expect_text(&filter, "~rust~henlo").await;
    Ok(())
}
