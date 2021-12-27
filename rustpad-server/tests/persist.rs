//! Tests to ensure that documents are persisted with SQLite.

use std::time::Duration;

use anyhow::Result;
use common::*;
use operational_transform::OperationSeq;
use rustpad_server::{
    database::{Database, PersistedDocument},
    server, ServerConfig,
};
use serde_json::json;
use tempfile::NamedTempFile;
use tokio::time;

pub mod common;

fn temp_sqlite_uri() -> Result<String> {
    Ok(format!(
        "sqlite://{}",
        NamedTempFile::new()?
            .into_temp_path()
            .as_os_str()
            .to_str()
            .expect("failed to get name of tempfile as &str")
    ))
}

#[tokio::test]
async fn test_database() -> Result<()> {
    pretty_env_logger::try_init().ok();

    let database = Database::new(&temp_sqlite_uri()?).await?;

    assert!(database.load("hello").await.is_err());
    assert!(database.load("world").await.is_err());

    let doc1 = PersistedDocument {
        text: "Hello Text".into(),
        language: None,
    };

    assert!(database.store("hello", &doc1).await.is_ok());
    assert_eq!(database.load("hello").await?, doc1);
    assert!(database.load("world").await.is_err());

    let doc2 = PersistedDocument {
        text: "print('World Text :)')".into(),
        language: Some("python".into()),
    };

    assert!(database.store("world", &doc2).await.is_ok());
    assert_eq!(database.load("hello").await?, doc1);
    assert_eq!(database.load("world").await?, doc2);

    assert!(database.store("hello", &doc2).await.is_ok());
    assert_eq!(database.load("hello").await?, doc2);

    Ok(())
}

#[tokio::test]
async fn test_persist() -> Result<()> {
    pretty_env_logger::try_init().ok();

    let filter = server(ServerConfig {
        expiry_days: 2,
        database: Some(Database::new(&temp_sqlite_uri()?).await?),
    });

    expect_text(&filter, "persist", "").await;

    let mut client = connect(&filter, "persist").await?;
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
    expect_text(&filter, "persist", "hello").await;

    let hour = Duration::from_secs(3600);
    time::pause();
    time::advance(47 * hour).await;
    expect_text(&filter, "persist", "hello").await;

    // Give SQLite some time to actually update the database.
    time::resume();
    time::sleep(Duration::from_millis(50)).await;
    time::pause();

    time::advance(3 * hour).await;
    expect_text(&filter, "persist", "hello").await;

    for _ in 0..50 {
        time::advance(10000 * hour).await;
        expect_text(&filter, "persist", "hello").await;
    }

    Ok(())
}
