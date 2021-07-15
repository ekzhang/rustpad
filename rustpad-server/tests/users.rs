//! Tests for synchronization of user presence.

use anyhow::Result;
use common::*;
use rustpad_server::{server, ServerConfig};
use serde_json::json;

pub mod common;

#[tokio::test]
async fn test_two_users() -> Result<()> {
    pretty_env_logger::try_init().ok();
    let filter = server(ServerConfig::default());

    let mut client = connect(&filter, "foobar").await?;
    assert_eq!(client.recv().await?, json!({ "Identity": 0 }));

    let alice = json!({
        "name": "Alice",
        "hue": 42
    });
    client.send(&json!({ "ClientInfo": alice })).await;

    let alice_info = json!({
        "UserInfo": {
            "id": 0,
            "info": alice
        }
    });
    assert_eq!(client.recv().await?, alice_info);

    let mut client2 = connect(&filter, "foobar").await?;
    assert_eq!(client2.recv().await?, json!({ "Identity": 1 }));
    assert_eq!(client2.recv().await?, alice_info);

    let bob = json!({
        "name": "Bob",
        "hue": 96
    });
    client2.send(&json!({ "ClientInfo": bob })).await;

    let bob_info = json!({
        "UserInfo": {
            "id": 1,
            "info": bob
        }
    });
    assert_eq!(client2.recv().await?, bob_info);
    assert_eq!(client.recv().await?, bob_info);

    Ok(())
}

#[tokio::test]
async fn test_invalid_user() -> Result<()> {
    pretty_env_logger::try_init().ok();
    let filter = server(ServerConfig::default());

    let mut client = connect(&filter, "foobar").await?;
    assert_eq!(client.recv().await?, json!({ "Identity": 0 }));

    let alice = json!({ "name": "Alice" }); // no hue
    client.send(&json!({ "ClientInfo": alice })).await;
    client.recv_closed().await?;

    Ok(())
}

#[tokio::test]
async fn test_leave_rejoin() -> Result<()> {
    pretty_env_logger::try_init().ok();
    let filter = server(ServerConfig::default());

    let mut client = connect(&filter, "foobar").await?;
    assert_eq!(client.recv().await?, json!({ "Identity": 0 }));

    let alice = json!({
        "name": "Alice",
        "hue": 42
    });
    client.send(&json!({ "ClientInfo": alice })).await;

    let alice_info = json!({
        "UserInfo": {
            "id": 0,
            "info": alice
        }
    });
    assert_eq!(client.recv().await?, alice_info);

    client.send(&json!({ "Invalid": "please close" })).await;
    client.recv_closed().await?;

    let mut client2 = connect(&filter, "foobar").await?;
    assert_eq!(client2.recv().await?, json!({ "Identity": 1 }));

    let bob = json!({
        "name": "Bob",
        "hue": 96
    });
    client2.send(&json!({ "ClientInfo": bob })).await;

    let bob_info = json!({
        "UserInfo": {
            "id": 1,
            "info": bob
        }
    });
    assert_eq!(client2.recv().await?, bob_info);

    Ok(())
}

#[tokio::test]
async fn test_cursors() -> Result<()> {
    pretty_env_logger::try_init().ok();
    let filter = server(ServerConfig::default());

    let mut client = connect(&filter, "foobar").await?;
    assert_eq!(client.recv().await?, json!({ "Identity": 0 }));

    let cursors = json!({
        "cursors": [4, 6, 7],
        "selections": [[5, 10], [3, 4]]
    });
    client.send(&json!({ "CursorData": cursors })).await;

    let cursors_resp = json!({
        "UserCursor": {
            "id": 0,
            "data": cursors
        }
    });
    assert_eq!(client.recv().await?, cursors_resp);

    let mut client2 = connect(&filter, "foobar").await?;
    assert_eq!(client2.recv().await?, json!({ "Identity": 1 }));
    assert_eq!(client2.recv().await?, cursors_resp);

    let cursors2 = json!({
        "cursors": [10],
        "selections": []
    });
    client2.send(&json!({ "CursorData": cursors2 })).await;

    let cursors2_resp = json!({
        "UserCursor": {
            "id": 1,
            "data": cursors2
        }
    });
    assert_eq!(client2.recv().await?, cursors2_resp);
    assert_eq!(client.recv().await?, cursors2_resp);

    client.send(&json!({ "Invalid": "please close" })).await;
    client.recv_closed().await?;

    let msg = json!({
        "Edit": {
            "revision": 0,
            "operation": ["a"]
        }
    });
    client2.send(&msg).await;

    let mut client3 = connect(&filter, "foobar").await?;
    assert_eq!(client3.recv().await?, json!({ "Identity": 2 }));
    client3.recv().await?;

    let transformed_cursors2_resp = json!({
        "UserCursor": {
            "id": 1,
            "data": {
                "cursors": [11],
                "selections": []
            }
        }
    });
    assert_eq!(client3.recv().await?, transformed_cursors2_resp);

    Ok(())
}
