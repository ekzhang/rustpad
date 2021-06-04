//! Tests for synchronization of user presence.

use anyhow::Result;
use common::*;
use rustpad_server::server;
use serde_json::json;

pub mod common;

#[tokio::test]
async fn test_two_users() -> Result<()> {
    pretty_env_logger::try_init().ok();
    let filter = server();

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
    let filter = server();

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
    let filter = server();

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
