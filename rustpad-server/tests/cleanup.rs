//! Tests to ensure that documents are garbage collected.

use std::time::Duration;

use anyhow::Result;
use common::*;
use operational_transform::OperationSeq;
use rustpad_server::server;
use serde_json::json;
use tokio::time;

pub mod common;

#[tokio::test]
async fn test_cleanup() -> Result<()> {
    pretty_env_logger::try_init().ok();
    let filter = server();

    expect_text(&filter, "old", "").await;

    let mut client = connect(&filter, "old").await?;
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
    client.send(&msg).await;

    let msg = client.recv().await?;
    msg.get("History")
        .expect("should receive history operation");
    expect_text(&filter, "old", "hello").await;

    let hour = Duration::from_secs(3600);
    time::pause();
    time::advance(23 * hour).await;
    expect_text(&filter, "old", "hello").await;

    time::advance(3 * hour).await;
    expect_text(&filter, "old", "").await;

    Ok(())
}
