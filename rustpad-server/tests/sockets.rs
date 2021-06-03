//! Basic tests for real-time collaboration.

use std::time::Duration;

use anyhow::Result;
use common::*;
use log::info;
use operational_transform::OperationSeq;
use rustpad_server::server;
use serde_json::json;
use tokio::time;

pub mod common;

#[tokio::test]
async fn test_single_operation() -> Result<()> {
    pretty_env_logger::try_init().ok();
    let filter = server();

    expect_text(&filter, "foobar", "").await;

    let mut client = connect(&filter, "foobar").await?;
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

    expect_text(&filter, "foobar", "hello").await;
    Ok(())
}

#[tokio::test]
async fn test_invalid_operation() -> Result<()> {
    pretty_env_logger::try_init().ok();
    let filter = server();

    expect_text(&filter, "foobar", "").await;

    let mut client = connect(&filter, "foobar").await?;
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
    let mut client = connect(&filter, "foobar").await?;
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
    expect_text(&filter, "foobar", "henlo").await;

    // Connect the second client
    let mut client2 = connect(&filter, "foobar").await?;
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

    expect_text(&filter, "foobar", "~rust~henlo").await;
    Ok(())
}

#[tokio::test]
async fn test_set_language() -> Result<()> {
    pretty_env_logger::try_init().ok();
    let filter = server();

    let mut client = connect(&filter, "foobar").await?;
    let msg = client.recv().await?;
    assert_eq!(msg, json!({ "Identity": 0 }));

    let msg = json!({ "SetLanguage": "javascript" });
    client.send(&msg).await;

    let msg = client.recv().await?;
    assert_eq!(msg, json!({ "Language": "javascript" }));

    let mut client2 = connect(&filter, "foobar").await?;
    let msg = client2.recv().await?;
    assert_eq!(msg, json!({ "Identity": 1 }));
    let msg = client2.recv().await?;
    assert_eq!(msg, json!({ "Language": "javascript" }));

    let msg = json!({ "SetLanguage": "python" });
    client2.send(&msg).await;

    let msg = client.recv().await?;
    assert_eq!(msg, json!({ "Language": "python" }));
    let msg = client2.recv().await?;
    assert_eq!(msg, json!({ "Language": "python" }));

    expect_text(&filter, "foobar", "").await;
    Ok(())
}
