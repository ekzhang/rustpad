//! Tests for running code through go-judge.

use std::sync::Arc;
use std::time::Duration;

use anyhow::{bail, Result};
use common::go_judge::*;
use common::*;
use operational_transform::OperationSeq;
use rustpad_server::{runner::GoJudgeClient, server, ServerConfig};
use serde_json::{json, Value};
use warp::{filters::BoxedFilter, Reply as WarpReply};

pub mod common;

/// A server config pointing at the fake, with short timeouts.
fn config(fake: &FakeJudge) -> ServerConfig {
    ServerConfig {
        runner: Some(
            GoJudgeClient::with_timeouts(
                &fake.url,
                None,
                Duration::from_secs(1),
                Duration::from_millis(300),
            )
            .unwrap(),
        ),
        ..ServerConfig::default()
    }
}

/// Reads messages until a non-empty `Runtimes` arrives, and returns its list.
/// The first `Runtimes` may be empty if the server hasn't probed yet.
async fn expect_runtimes(client: &mut JsonSocket) -> Result<Value> {
    loop {
        let msg = client.recv().await?;
        match msg.get("Runtimes") {
            Some(list) if list.as_array().is_some_and(|a| !a.is_empty()) => return Ok(list.clone()),
            Some(_) => continue,
            None => bail!("expected Runtimes, got {msg}"),
        }
    }
}

#[tokio::test]
async fn test_runtimes_on_connect() -> Result<()> {
    pretty_env_logger::try_init().ok();
    let fake = start(default_handler("")).await;
    let filter = server(config(&fake));

    let mut client = connect(&filter, "pad").await?;
    assert_eq!(client.recv().await?, json!({ "Identity": 0 }));
    let runtimes = expect_runtimes(&mut client).await?;
    let runtimes = runtimes.as_array().unwrap();

    assert!(runtimes
        .contains(&json!({ "language": "python", "runtime": "python", "version": "3.13.5" })));
    assert!(runtimes.contains(&json!({ "language": "cpp", "runtime": "g++", "version": "14.2.0" })));
    assert!(runtimes
        .contains(&json!({ "language": "kotlin", "runtime": "kotlin", "version": "2.2.21" })));
    assert!(!runtimes.iter().any(|r| r["language"] == "markdown"));
    Ok(())
}

#[tokio::test]
async fn test_no_runtimes_when_off() -> Result<()> {
    pretty_env_logger::try_init().ok();
    let filter = server(ServerConfig::default());

    let mut client = connect(&filter, "pad").await?;
    assert_eq!(client.recv().await?, json!({ "Identity": 0 }));

    // The next message must be the reply to ClientInfo, not Runtimes.
    let alice = json!({ "name": "Alice", "hue": 42 });
    client.send(&json!({ "ClientInfo": alice })).await;
    assert_eq!(
        client.recv().await?,
        json!({ "UserInfo": { "id": 0, "info": alice } })
    );
    Ok(())
}

#[tokio::test]
async fn test_failed_probe_hides_language() -> Result<()> {
    pretty_env_logger::try_init().ok();
    let inner = default_handler("");
    let handler: Handler = Arc::new(move |cmd: &Value| {
        if cmd["args"][0]
            .as_str()
            .is_some_and(|a| a.ends_with("kotlinc"))
        {
            return Reply::Status(500);
        }
        inner(cmd)
    });
    let fake = start(handler).await;
    let filter = server(config(&fake));

    let mut client = connect(&filter, "pad").await?;
    assert_eq!(client.recv().await?, json!({ "Identity": 0 }));
    let runtimes = expect_runtimes(&mut client).await?;
    let runtimes = runtimes.as_array().unwrap();

    assert!(runtimes.iter().any(|r| r["language"] == "python"));
    assert!(!runtimes.iter().any(|r| r["language"] == "kotlin"));
    Ok(())
}

/// Connects the first user and sets the pad's language and text.
async fn open_pad(
    filter: &BoxedFilter<(impl WarpReply + 'static,)>,
    id: &str,
    language: &str,
    text: &str,
) -> Result<JsonSocket> {
    let mut client = connect(filter, id).await?;
    assert_eq!(client.recv().await?, json!({ "Identity": 0 }));
    expect_runtimes(&mut client).await?;
    client.send(&json!({ "SetLanguage": language })).await;
    assert_eq!(client.recv().await?, json!({ "Language": language }));
    let mut operation = OperationSeq::default();
    operation.insert(text);
    client
        .send(&json!({ "Edit": { "revision": 0, "operation": operation } }))
        .await;
    assert!(client.recv().await?.get("History").is_some());
    Ok(client)
}

/// Connects another user to a pad that already has text and a language.
async fn join_pad(
    filter: &BoxedFilter<(impl WarpReply + 'static,)>,
    id: &str,
    user: u64,
) -> Result<JsonSocket> {
    let mut client = connect(filter, id).await?;
    assert_eq!(client.recv().await?, json!({ "Identity": user }));
    assert!(client.recv().await?.get("History").is_some());
    assert!(client.recv().await?.get("Language").is_some());
    expect_runtimes(&mut client).await?;
    Ok(client)
}

/// The commands that ran or compiled code. Probes carry no `copyIn`.
fn code_requests(fake: &FakeJudge) -> Vec<Value> {
    fake.requests
        .lock()
        .iter()
        .filter(|cmd| cmd.get("copyIn").is_some())
        .cloned()
        .collect()
}

#[tokio::test]
async fn test_shared_run() -> Result<()> {
    pretty_env_logger::try_init().ok();
    let fake = start(default_handler("hi\n")).await;
    let filter = server(config(&fake));
    let mut alice = open_pad(&filter, "pad", "python", "print('hi')").await?;
    let mut bob = join_pad(&filter, "pad", 1).await?;

    alice.send(&json!("Run")).await;

    for client in [&mut alice, &mut bob] {
        assert_eq!(client.recv().await?, json!({ "RunStarted": { "id": 0 } }));
        let finished = client.recv().await?;
        let finished = &finished["RunFinished"];
        assert_eq!(finished["id"], 0);
        assert_eq!(finished["language"], "python");
        assert_eq!(finished["runtime"], "python");
        assert_eq!(finished["version"], "3.13.5");
        assert_eq!(finished["compile"], Value::Null);
        assert_eq!(
            finished["run"],
            json!({
                "stdout": "hi\n",
                "stderr": "",
                "code": 0,
                "signal": null,
                "status": null,
                "truncated": false
            })
        );
        assert!(finished["duration_ms"].is_u64());
    }

    let requests = code_requests(&fake);
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0]["copyIn"]["main.py"]["content"], "print('hi')");
    assert_eq!(requests[0]["files"][0], json!({ "content": "" }));
    Ok(())
}

#[tokio::test]
async fn test_compile_error() -> Result<()> {
    pretty_env_logger::try_init().ok();
    let inner = default_handler("");
    let handler: Handler = Arc::new(move |cmd: &Value| {
        if cmd.get("copyOutCached").is_some() {
            return Reply::Results(json!([result("Nonzero Exit Status", 1, "", "error")]));
        }
        inner(cmd)
    });
    let fake = start(handler).await;
    let filter = server(config(&fake));
    let mut alice = open_pad(&filter, "pad", "cpp", "int main() {").await?;

    alice.send(&json!("Run")).await;
    assert_eq!(alice.recv().await?, json!({ "RunStarted": { "id": 0 } }));
    let finished = alice.recv().await?;
    assert_eq!(finished["RunFinished"]["runtime"], "g++");
    assert_eq!(finished["RunFinished"]["compile"]["stderr"], "error");
    assert_eq!(finished["RunFinished"]["run"], Value::Null);
    assert_eq!(code_requests(&fake).len(), 1);
    Ok(())
}

#[tokio::test]
async fn test_compiled_run_finishes() -> Result<()> {
    pretty_env_logger::try_init().ok();
    let fake = start(default_handler("hi\n")).await;
    let filter = server(config(&fake));
    let mut alice = open_pad(&filter, "pad", "cpp", "int main() {}").await?;

    alice.send(&json!("Run")).await;
    assert_eq!(alice.recv().await?, json!({ "RunStarted": { "id": 0 } }));
    let finished = alice.recv().await?;
    assert_eq!(finished["RunFinished"]["compile"]["code"], 0);
    assert_eq!(finished["RunFinished"]["run"]["stdout"], "hi\n");
    assert_eq!(*fake.deleted.lock(), ["FID1"]);
    Ok(())
}

#[tokio::test]
async fn test_two_pads_run_concurrently() -> Result<()> {
    pretty_env_logger::try_init().ok();
    let inner = default_handler("");
    // Each run prints the name of its source file.
    let handler: Handler = Arc::new(move |cmd: &Value| match cmd["copyIn"].as_object() {
        Some(files) => {
            let names: Vec<_> = files.keys().map(String::as_str).collect();
            let stdout = names.join(" ");
            let value = json!([result("Accepted", 0, &stdout, "")]);
            Reply::Slow(Duration::from_millis(150), value)
        }
        None => inner(cmd),
    });
    let fake = start(handler).await;
    let filter = server(config(&fake));
    let mut a = open_pad(&filter, "a", "python", "print(1)").await?;
    let mut b = open_pad(&filter, "b", "javascript", "console.log(1)").await?;

    a.send(&json!("Run")).await;
    b.send(&json!("Run")).await;

    let pads = [
        (&mut a, "python", "main.py"),
        (&mut b, "javascript", "main.js"),
    ];
    for (client, language, file) in pads {
        assert_eq!(client.recv().await?, json!({ "RunStarted": { "id": 0 } }));
        let finished = client.recv().await?;
        let finished = &finished["RunFinished"];
        assert_eq!(finished["language"], language);
        let stdout = finished["run"]["stdout"].as_str().unwrap_or_default();
        assert!(stdout.contains(file), "{finished}");
    }

    let requests = code_requests(&fake);
    assert_eq!(requests.len(), 2);
    assert!(requests
        .iter()
        .any(|c| c["copyIn"].get("main.py").is_some()));
    assert!(requests
        .iter()
        .any(|c| c["copyIn"].get("main.js").is_some()));
    Ok(())
}

#[tokio::test]
async fn test_busy_run_ignored() -> Result<()> {
    pretty_env_logger::try_init().ok();
    let inner = default_handler("once");
    let handler: Handler = Arc::new(move |cmd: &Value| match inner(cmd) {
        Reply::Results(value) if cmd.get("copyIn").is_some() => {
            Reply::Slow(Duration::from_millis(150), value)
        }
        other => other,
    });
    let fake = start(handler).await;
    let filter = server(config(&fake));
    let mut alice = open_pad(&filter, "pad", "python", "print('once')").await?;

    alice.send(&json!("Run")).await;
    alice.send(&json!("Run")).await;
    assert_eq!(alice.recv().await?, json!({ "RunStarted": { "id": 0 } }));
    assert!(alice.recv().await?.get("RunFinished").is_some());

    // The second Run arrived during the first and must not reach go-judge.
    assert_eq!(code_requests(&fake).len(), 1);
    Ok(())
}

#[tokio::test]
async fn test_language_change_during_run() -> Result<()> {
    pretty_env_logger::try_init().ok();
    let inner = default_handler("hi\n");
    let handler: Handler = Arc::new(move |cmd: &Value| match inner(cmd) {
        Reply::Results(value) if cmd.get("copyIn").is_some() => {
            Reply::Slow(Duration::from_millis(150), value)
        }
        other => other,
    });
    let fake = start(handler).await;
    let filter = server(config(&fake));
    let mut alice = open_pad(&filter, "pad", "python", "print('hi')").await?;

    alice.send(&json!("Run")).await;
    alice.send(&json!({ "SetLanguage": "cpp" })).await;
    assert_eq!(alice.recv().await?, json!({ "RunStarted": { "id": 0 } }));
    assert_eq!(alice.recv().await?, json!({ "Language": "cpp" }));
    let finished = alice.recv().await?;
    assert_eq!(finished["RunFinished"]["language"], "python");
    assert_eq!(finished["RunFinished"]["runtime"], "python");
    Ok(())
}

#[tokio::test]
async fn test_unsupported_language() -> Result<()> {
    pretty_env_logger::try_init().ok();
    let fake = start(default_handler("")).await;
    let filter = server(config(&fake));
    let mut alice = open_pad(&filter, "pad", "markdown", "# hi").await?;

    alice.send(&json!("Run")).await;
    assert_eq!(
        alice.recv().await?,
        json!({ "RunFailed": { "id": 0, "error": "No runtime for markdown" } })
    );
    assert!(code_requests(&fake).is_empty());
    Ok(())
}

#[tokio::test]
async fn test_code_too_large() -> Result<()> {
    pretty_env_logger::try_init().ok();
    let fake = start(default_handler("")).await;
    let filter = server(config(&fake));
    let mut alice = open_pad(&filter, "pad", "python", &"x".repeat(64 * 1024 + 1)).await?;

    alice.send(&json!("Run")).await;
    assert_eq!(
        alice.recv().await?,
        json!({ "RunFailed": { "id": 0, "error": "Code too large to run (64 KiB limit)" } })
    );
    assert!(code_requests(&fake).is_empty());
    Ok(())
}

#[tokio::test]
async fn test_run_without_language() -> Result<()> {
    pretty_env_logger::try_init().ok();
    let fake = start(default_handler("")).await;
    let filter = server(config(&fake));
    let mut alice = connect(&filter, "pad").await?;
    assert_eq!(alice.recv().await?, json!({ "Identity": 0 }));
    expect_runtimes(&mut alice).await?;

    alice.send(&json!("Run")).await;
    assert_eq!(
        alice.recv().await?,
        json!({ "RunFailed": { "id": 0, "error": "No runtime for plaintext" } })
    );
    Ok(())
}

#[tokio::test]
async fn test_runner_error() -> Result<()> {
    pretty_env_logger::try_init().ok();
    let inner = default_handler("");
    let handler: Handler = Arc::new(move |cmd: &Value| {
        if cmd.get("copyIn").is_some() {
            return Reply::Status(500);
        }
        inner(cmd)
    });
    let fake = start(handler).await;
    let filter = server(config(&fake));
    let mut alice = open_pad(&filter, "pad", "python", "1").await?;

    alice.send(&json!("Run")).await;
    assert_eq!(alice.recv().await?, json!({ "RunStarted": { "id": 0 } }));
    assert_eq!(
        alice.recv().await?,
        json!({ "RunFailed": { "id": 0, "error": "Code runner is unavailable" } })
    );
    Ok(())
}

#[tokio::test]
async fn test_runner_timeout() -> Result<()> {
    pretty_env_logger::try_init().ok();
    let inner = default_handler("late");
    let handler: Handler = Arc::new(move |cmd: &Value| match inner(cmd) {
        Reply::Results(value) if cmd.get("copyIn").is_some() => {
            Reply::Slow(Duration::from_millis(800), value)
        }
        other => other,
    });
    let fake = start(handler).await;
    let filter = server(config(&fake));
    let mut alice = open_pad(&filter, "pad", "python", "1").await?;

    alice.send(&json!("Run")).await;
    assert_eq!(alice.recv().await?, json!({ "RunStarted": { "id": 0 } }));
    assert_eq!(
        alice.recv().await?,
        json!({ "RunFailed": { "id": 0, "error": "Code runner timed out" } })
    );
    Ok(())
}

#[tokio::test]
async fn test_late_joiner_sees_last_result() -> Result<()> {
    pretty_env_logger::try_init().ok();
    let fake = start(default_handler("hi\n")).await;
    let filter = server(config(&fake));
    let mut alice = open_pad(&filter, "pad", "python", "print('hi')").await?;

    alice.send(&json!("Run")).await;
    alice.recv().await?; // RunStarted
    let finished = alice.recv().await?;

    let mut carol = join_pad(&filter, "pad", 1).await?;
    assert_eq!(carol.recv().await?, finished);
    Ok(())
}

#[tokio::test]
async fn test_late_joiner_sees_run_in_progress() -> Result<()> {
    pretty_env_logger::try_init().ok();
    let inner = default_handler("hi\n");
    let handler: Handler = Arc::new(move |cmd: &Value| match inner(cmd) {
        Reply::Results(value) if cmd.get("copyIn").is_some() => {
            Reply::Slow(Duration::from_millis(150), value)
        }
        other => other,
    });
    let fake = start(handler).await;
    let filter = server(config(&fake));
    let mut alice = open_pad(&filter, "pad", "python", "print('hi')").await?;

    alice.send(&json!("Run")).await;
    assert_eq!(alice.recv().await?, json!({ "RunStarted": { "id": 0 } }));

    let mut carol = join_pad(&filter, "pad", 1).await?;
    assert_eq!(carol.recv().await?, json!({ "RunStarted": { "id": 0 } }));
    assert!(carol.recv().await?.get("RunFinished").is_some());
    Ok(())
}

#[tokio::test]
async fn test_runner_leaves_mid_run() -> Result<()> {
    pretty_env_logger::try_init().ok();
    let inner = default_handler("hi\n");
    let handler: Handler = Arc::new(move |cmd: &Value| match inner(cmd) {
        Reply::Results(value) if cmd.get("copyIn").is_some() => {
            Reply::Slow(Duration::from_millis(150), value)
        }
        other => other,
    });
    let fake = start(handler).await;
    let filter = server(config(&fake));
    let mut alice = open_pad(&filter, "pad", "python", "print('hi')").await?;
    let mut bob = join_pad(&filter, "pad", 1).await?;

    alice.send(&json!("Run")).await;
    assert_eq!(bob.recv().await?, json!({ "RunStarted": { "id": 0 } }));
    drop(alice);

    // Bob gets Alice's departure and the result, in either order.
    let first = bob.recv().await?;
    let second = bob.recv().await?;
    assert!(
        first.get("RunFinished").is_some() || second.get("RunFinished").is_some(),
        "no RunFinished in {first} / {second}"
    );
    Ok(())
}

#[tokio::test]
async fn test_run_ignored_when_off() -> Result<()> {
    pretty_env_logger::try_init().ok();
    let filter = server(ServerConfig::default());
    let mut client = connect(&filter, "pad").await?;
    assert_eq!(client.recv().await?, json!({ "Identity": 0 }));

    client.send(&json!("Run")).await;
    let alice = json!({ "name": "Alice", "hue": 42 });
    client.send(&json!({ "ClientInfo": alice })).await;
    assert_eq!(
        client.recv().await?,
        json!({ "UserInfo": { "id": 0, "info": alice } })
    );
    Ok(())
}
