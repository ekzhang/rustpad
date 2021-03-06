//! Tests for Unicode support and correct cursor transformation.

pub mod common;

use anyhow::Result;
use common::*;
use log::info;
use operational_transform::OperationSeq;
use rustpad_server::{server, ServerConfig};
use serde_json::json;

#[tokio::test]
async fn test_unicode_length() -> Result<()> {
    pretty_env_logger::try_init().ok();
    let filter = server(ServerConfig::default());

    expect_text(&filter, "unicode", "").await;

    let mut client = connect(&filter, "unicode").await?;
    let msg = client.recv().await?;
    assert_eq!(msg, json!({ "Identity": 0 }));

    let mut operation = OperationSeq::default();
    operation.insert("hðeðlð¨âð¨âð¦âð¦lo");
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
                    { "id": 0, "operation": ["hðeðlð¨âð¨âð¦âð¦lo"] }
                ]
            }
        })
    );

    info!("testing that text length is equal to number of Unicode code points...");
    let mut operation = OperationSeq::default();
    operation.delete(14);
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
                    { "id": 0, "operation": [-14] }
                ]
            }
        })
    );

    expect_text(&filter, "unicode", "").await;

    Ok(())
}

#[tokio::test]
async fn test_multiple_operations() -> Result<()> {
    pretty_env_logger::try_init().ok();
    let filter = server(ServerConfig::default());

    expect_text(&filter, "unicode", "").await;

    let mut client = connect(&filter, "unicode").await?;
    let msg = client.recv().await?;
    assert_eq!(msg, json!({ "Identity": 0 }));

    let mut operation = OperationSeq::default();
    operation.insert("ðððð¨âð¨âð¦âð¦"); // Emoticons and Cuneiform
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
                    { "id": 0, "operation": ["ðððð¨âð¨âð¦âð¦"] }
                ]
            }
        })
    );

    let mut operation = OperationSeq::default();
    operation.insert("ð¯ââï¸");
    operation.retain(3);
    operation.insert("ð£ð¤ð¥"); // Ancient Greek numbers
    operation.retain(7);
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
                    { "id": 0, "operation": ["ð¯ââï¸", 3, "ð£ð¤ð¥", 7] }
                ]
            }
        })
    );

    expect_text(&filter, "unicode", "ð¯ââï¸ðððð£ð¤ð¥ð¨âð¨âð¦âð¦").await;

    let mut operation = OperationSeq::default();
    operation.retain(2);
    operation.insert("hÌ·ÌÍÌÌÌÌÌÍÌÌ¤eÌ¶ÌÌÌÌÍÌ½ÌÌÌ lÌ¸ÍÌÍÍÍ Ì¥lÌ¸Í Ì Ì£ÌÌ¥Ì¨oÌ´ÌÌÌÍÌÍÍÌ½ÍÍÌÍÍÍÌ³ÌÌªÌÌ±Ì°Ì¥ÌÌ Ì¶ÍÍÍÍÌÍÌ£ÌÍÌ¡Ì¯ÍÌÌ wÌ·ÌÌÌ¿Ì¿ÌÌ¾ÍÍÍÌÌÍÌÌªÌ¨ÍÌ¤ÍÍÌÍÌÌ­Ì¨Ì¯ÌoÌ´ÌÍ ÌÍÌÌÍÍÌÌÍÌÌ¥Ì¤rÌ¶ÌÍÌ¨Ì¦ÍÌÌ®Ì¦ÍÍÌ©Ì¡Ì¢ÍÌÌlÌ¶ÍÍÍÌÍÍÍ ÍÍÌÌÌÍÍÌ¬Ì¤ÍÌ¡Ì¢ÌdÌµÍÌÌÌÍ ÍÌÌÌÍÌÍÌÌÍÌ¯ÍÌ­ÍÌ©ÍÌÌÍ"); // Lots of ligatures
    operation.retain(8);
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
                "start": 2,
                "operations": [
                    { "id": 0, "operation": [6, "hÌ·ÌÍÌÌÌÌÌÍÌÌ¤eÌ¶ÌÌÌÌÍÌ½ÌÌÌ lÌ¸ÍÌÍÍÍ Ì¥lÌ¸Í Ì Ì£ÌÌ¥Ì¨oÌ´ÌÌÌÍÌÍÍÌ½ÍÍÌÍÍÍÌ³ÌÌªÌÌ±Ì°Ì¥ÌÌ Ì¶ÍÍÍÍÌÍÌ£ÌÍÌ¡Ì¯ÍÌÌ wÌ·ÌÌÌ¿Ì¿ÌÌ¾ÍÍÍÌÌÍÌÌªÌ¨ÍÌ¤ÍÍÌÍÌÌ­Ì¨Ì¯ÌoÌ´ÌÍ ÌÍÌÌÍÍÌÌÍÌÌ¥Ì¤rÌ¶ÌÍÌ¨Ì¦ÍÌÌ®Ì¦ÍÍÌ©Ì¡Ì¢ÍÌÌlÌ¶ÍÍÍÌÍÍÍ ÍÍÌÌÌÍÍÌ¬Ì¤ÍÌ¡Ì¢ÌdÌµÍÌÌÌÍ ÍÌÌÌÍÌÍÌÌÍÌ¯ÍÌ­ÍÌ©ÍÌÌÍ", 11] }
                ]
            }
        })
    );

    expect_text(&filter, "unicode", "ð¯ââï¸ððhÌ·ÌÍÌÌÌÌÌÍÌÌ¤eÌ¶ÌÌÌÌÍÌ½ÌÌÌ lÌ¸ÍÌÍÍÍ Ì¥lÌ¸Í Ì Ì£ÌÌ¥Ì¨oÌ´ÌÌÌÍÌÍÍÌ½ÍÍÌÍÍÍÌ³ÌÌªÌÌ±Ì°Ì¥ÌÌ Ì¶ÍÍÍÍÌÍÌ£ÌÍÌ¡Ì¯ÍÌÌ wÌ·ÌÌÌ¿Ì¿ÌÌ¾ÍÍÍÌÌÍÌÌªÌ¨ÍÌ¤ÍÍÌÍÌÌ­Ì¨Ì¯ÌoÌ´ÌÍ ÌÍÌÌÍÍÌÌÍÌÌ¥Ì¤rÌ¶ÌÍÌ¨Ì¦ÍÌÌ®Ì¦ÍÍÌ©Ì¡Ì¢ÍÌÌlÌ¶ÍÍÍÌÍÍÍ ÍÍÌÌÌÍÍÌ¬Ì¤ÍÌ¡Ì¢ÌdÌµÍÌÌÌÍ ÍÌÌÌÍÌÍÌÌÍÌ¯ÍÌ­ÍÌ©ÍÌÌÍðð£ð¤ð¥ð¨âð¨âð¦âð¦").await;

    Ok(())
}

#[tokio::test]
async fn test_unicode_cursors() -> Result<()> {
    pretty_env_logger::try_init().ok();
    let filter = server(ServerConfig::default());

    let mut client = connect(&filter, "unicode").await?;
    assert_eq!(client.recv().await?, json!({ "Identity": 0 }));

    let mut operation = OperationSeq::default();
    operation.insert("ððð");
    let msg = json!({
        "Edit": {
            "revision": 0,
            "operation": operation
        }
    });
    info!("sending ClientMsg {}", msg);
    client.send(&msg).await;
    client.recv().await?;

    let cursors = json!({
        "cursors": [0, 1, 2, 3],
        "selections": [[0, 1], [2, 3]]
    });
    client.send(&json!({ "CursorData": cursors })).await;

    let cursors_resp = json!({
        "UserCursor": {
            "id": 0,
            "data": cursors
        }
    });
    assert_eq!(client.recv().await?, cursors_resp);

    let mut client2 = connect(&filter, "unicode").await?;
    assert_eq!(client2.recv().await?, json!({ "Identity": 1 }));
    client2.recv().await?;
    assert_eq!(client2.recv().await?, cursors_resp);

    let msg = json!({
        "Edit": {
            "revision": 0,
            "operation": ["ð"]
        }
    });
    client2.send(&msg).await;

    let mut client3 = connect(&filter, "unicode").await?;
    assert_eq!(client3.recv().await?, json!({ "Identity": 2 }));
    client3.recv().await?;

    let transformed_cursors_resp = json!({
        "UserCursor": {
            "id": 0,
            "data": {
                "cursors": [1, 2, 3, 4],
                "selections": [[1, 2], [3, 4]]
            }
        }
    });
    assert_eq!(client3.recv().await?, transformed_cursors_resp);

    Ok(())
}
