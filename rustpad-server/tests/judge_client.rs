//! Tests for the go-judge client against a fake go-judge.

use std::sync::Arc;
use std::time::Duration;

use common::go_judge::*;
use rustpad_server::runner::{GoJudgeClient, RunError, MAX_STREAM_BYTES};
use serde_json::{json, Value};

pub mod common;

fn client(url: &str) -> GoJudgeClient {
    GoJudgeClient::with_timeouts(
        url,
        None,
        Duration::from_secs(1),
        Duration::from_millis(300),
    )
    .unwrap()
}

fn results(value: Value) -> Handler {
    Arc::new(move |_| Reply::Results(json!([value.clone()])))
}

/// Answers every call with `reply`.
fn always(reply: Reply) -> Handler {
    Arc::new(move |_| reply.clone())
}

/// Runs python source against a handler that answers every call alike.
async fn run_python(handler: Handler) -> Result<rustpad_server::runner::RunStage, RunError> {
    let fake = start(handler).await;
    let out = client(&fake.url).execute("python", "pass").await?;
    Ok(out.run.unwrap())
}

#[tokio::test]
async fn test_execute_python_request() {
    let fake = start(default_handler("hi\n")).await;
    let output = client(&fake.url)
        .execute("python", "print(1)")
        .await
        .unwrap();

    assert!(output.compile.is_none());
    assert_eq!(output.run.unwrap().stdout, "hi\n");

    let requests = fake.requests.lock();
    assert_eq!(requests.len(), 1);
    let cmd = &requests[0];
    assert_eq!(cmd["args"], json!(["/usr/bin/python3", "main.py"]));
    assert_eq!(cmd["copyIn"]["main.py"]["content"], "print(1)");
    assert_eq!(cmd["cpuLimit"], 3_000_000_000u64);
    assert_eq!(cmd["clockLimit"], 5_000_000_000u64);
    assert_eq!(cmd["memoryLimit"], 268435456);
    assert_eq!(cmd["procLimit"], 64);
    assert_eq!(cmd["files"][0], json!({ "content": "" }));
    assert_eq!(cmd["files"][1], json!({ "name": "stdout", "max": 65536 }));
    assert_eq!(cmd["files"][2], json!({ "name": "stderr", "max": 65536 }));
    assert!(cmd["env"].as_array().unwrap().contains(&json!("HOME=/w")));
    assert!(cmd.get("copyOutCached").is_none());
}

#[tokio::test]
async fn test_execute_cpp_two_steps() {
    let fake = start(default_handler("out")).await;
    let output = client(&fake.url)
        .execute("cpp", "int main(){}")
        .await
        .unwrap();

    let compile = output.compile.unwrap();
    assert_eq!(compile.code, Some(0));
    assert_eq!(output.run.unwrap().stdout, "out");

    let requests = fake.requests.lock();
    assert_eq!(requests.len(), 2);
    assert_eq!(requests[0]["copyOutCached"], json!(["main?"]));
    assert_eq!(requests[0]["copyIn"]["main.cpp"]["content"], "int main(){}");
    assert_eq!(requests[1]["copyIn"]["main"], json!({ "fileId": "FID1" }));
    assert_eq!(requests[1]["args"], json!(["main"]));
    assert_eq!(*fake.deleted.lock(), vec!["FID1".to_string()]);
}

#[tokio::test]
async fn test_execute_java_names_file_after_class() {
    let fake = start(default_handler("")).await;
    let source = "public class Solution { public static void main(String[] a) {} }";
    client(&fake.url).execute("java", source).await.unwrap();

    let requests = fake.requests.lock();
    let args = requests[0]["args"].as_array().unwrap();
    assert_eq!(
        args[args.len() - 2..],
        [json!("Solution.java"), json!("Solution")]
    );
    assert_eq!(requests[0]["copyIn"]["Solution.java"]["content"], source);
    assert_eq!(
        requests[1]["copyIn"]["main.jar"],
        json!({ "fileId": "FID1" })
    );
}

#[tokio::test]
async fn test_execute_compile_error_skips_run() {
    let fake = start(always(Reply::Results(json!([result(
        "Nonzero Exit Status",
        1,
        "",
        "error"
    )]))))
    .await;
    let output = client(&fake.url).execute("cpp", "x").await.unwrap();
    assert!(output.run.is_none());
    let compile = output.compile.unwrap();
    assert_eq!(compile.code, Some(1));
    assert_eq!(compile.stderr, "error");
    assert_eq!(fake.requests.lock().len(), 1);
    assert!(fake.deleted.lock().is_empty());
}

#[tokio::test]
async fn test_execute_maps_statuses() {
    // (status, exitStatus, code, signal, status text, truncated)
    let cases: Vec<(&str, i64, Option<i32>, Option<&str>, Option<&str>, bool)> = vec![
        ("Accepted", 0, Some(0), None, None, false),
        ("Nonzero Exit Status", 3, Some(3), None, None, false),
        ("Output Limit Exceeded", 0, Some(0), None, None, true),
        (
            "Time Limit Exceeded",
            -1,
            None,
            Some("SIGKILL"),
            Some("Time limit exceeded"),
            false,
        ),
        (
            "Memory Limit Exceeded",
            -1,
            None,
            Some("SIGKILL"),
            Some("Memory limit exceeded"),
            false,
        ),
        ("Signalled", 11, None, Some("SIGSEGV"), None, false),
        ("Signalled", 42, None, Some("signal 42"), None, false),
        (
            "Dangerous Syscall",
            31,
            None,
            None,
            Some("Blocked system call"),
            false,
        ),
    ];
    for (status, exit, code, signal, text, truncated) in cases {
        let run = run_python(results(result(status, exit, "o", "e")))
            .await
            .unwrap();
        let label = format!("{status} {exit}");
        assert_eq!(run.code, code, "{label}");
        assert_eq!(run.signal.as_deref(), signal, "{label}");
        assert_eq!(run.status.as_deref(), text, "{label}");
        assert_eq!(run.truncated, truncated, "{label}");
        assert_eq!(
            (run.stdout.as_str(), run.stderr.as_str()),
            ("o", "e"),
            "{label}"
        );
    }
}

#[tokio::test]
async fn test_execute_signal_names() {
    let names = [
        (4, "SIGILL"),
        (6, "SIGABRT"),
        (7, "SIGBUS"),
        (8, "SIGFPE"),
        (9, "SIGKILL"),
        (13, "SIGPIPE"),
        (15, "SIGTERM"),
        (24, "SIGXCPU"),
        (25, "SIGXFSZ"),
    ];
    for (n, name) in names {
        let run = run_python(results(result("Signalled", n, "", "")))
            .await
            .unwrap();
        assert_eq!(run.signal.as_deref(), Some(name));
    }
}

#[tokio::test]
async fn test_execute_internal_error() {
    for status in ["Internal Error", "File Error", "Something New"] {
        let mut res = result(status, 0, "", "");
        res["error"] = json!("boom");
        let err = run_python(results(res)).await.unwrap_err();
        assert_eq!(err, RunError::InvalidResponse, "{status}");
    }
}

#[tokio::test]
async fn test_execute_missing_result_is_invalid() {
    let err = run_python(always(Reply::Results(json!([]))))
        .await
        .unwrap_err();
    assert_eq!(err, RunError::InvalidResponse);
}

#[tokio::test]
async fn test_execute_missing_cache_id_is_invalid() {
    let fake = start(always(Reply::Results(json!([result(
        "Accepted", 0, "", ""
    )]))))
    .await;
    let err = client(&fake.url).execute("cpp", "x").await.unwrap_err();
    assert_eq!(err, RunError::InvalidResponse);
    assert_eq!(fake.requests.lock().len(), 1);
}

#[tokio::test]
async fn test_execute_run_file_error_still_deletes() {
    let inner = default_handler("");
    let handler: Handler = Arc::new(move |cmd| {
        if cmd["copyIn"]["main"].is_object() {
            Reply::Results(json!([result("File Error", 0, "", "")]))
        } else {
            inner(cmd)
        }
    });
    let fake = start(handler).await;
    let err = client(&fake.url).execute("cpp", "x").await.unwrap_err();
    assert_eq!(err, RunError::InvalidResponse);
    assert_eq!(*fake.deleted.lock(), vec!["FID1".to_string()]);
}

#[tokio::test]
async fn test_execute_unknown_language() {
    let fake = start(default_handler("")).await;
    let err = client(&fake.url)
        .execute("markdown", "x")
        .await
        .unwrap_err();
    assert_eq!(err, RunError::Unavailable);
    assert!(fake.requests.lock().is_empty());
}

#[tokio::test]
async fn test_execute_truncates_output() {
    let run = run_python(results(result(
        "Accepted",
        0,
        &"a".repeat(MAX_STREAM_BYTES + 10),
        "",
    )))
    .await
    .unwrap();
    assert_eq!(run.stdout.len(), MAX_STREAM_BYTES);
    assert!(run.truncated);
}

#[tokio::test]
async fn test_execute_truncates_multibyte_output() {
    let run = run_python(results(result(
        "Accepted",
        0,
        &"😀".repeat(MAX_STREAM_BYTES / 4 + 1),
        "",
    )))
    .await
    .unwrap();
    assert!(run.stdout.len() <= MAX_STREAM_BYTES);
    assert_eq!(run.stdout.len() % 4, 0);
    assert!(run.truncated);
}

#[tokio::test]
async fn test_execute_server_error() {
    let err = run_python(always(Reply::Status(500))).await.unwrap_err();
    assert_eq!(err, RunError::Unavailable);
    assert_eq!(err.to_string(), "Code runner is unavailable");
}

#[tokio::test]
async fn test_execute_timeout() {
    let slow = Reply::Slow(
        Duration::from_millis(800),
        json!([result("Accepted", 0, "", "")]),
    );
    let err = run_python(always(slow)).await.unwrap_err();
    assert_eq!(err, RunError::TimedOut);
}

#[tokio::test]
async fn test_execute_oversized_body() {
    let err = run_python(always(Reply::Raw("x".repeat(5 * 1024 * 1024))))
        .await
        .unwrap_err();
    assert_eq!(err, RunError::InvalidResponse);
}

#[tokio::test]
async fn test_execute_invalid_json() {
    let err = run_python(always(Reply::Raw("not json".into())))
        .await
        .unwrap_err();
    assert_eq!(err, RunError::InvalidResponse);
}

#[tokio::test]
async fn test_connection_refused() {
    let err = client("http://127.0.0.1:1")
        .execute("python", "x")
        .await
        .unwrap_err();
    assert_eq!(err, RunError::Unavailable);
}

#[test]
fn test_rejects_invalid_url() {
    assert!(GoJudgeClient::new("not a url", None).is_err());
}

#[tokio::test]
async fn test_sends_bearer_token() {
    let fake = start(default_handler("")).await;
    let with = GoJudgeClient::new(&fake.url, Some("secret".into())).unwrap();
    with.execute("python", "x").await.unwrap();
    assert_eq!(*fake.auth.lock(), vec![Some("Bearer secret".to_string())]);

    let fake = start(default_handler("")).await;
    GoJudgeClient::new(&fake.url, None)
        .unwrap()
        .execute("python", "x")
        .await
        .unwrap();
    assert_eq!(*fake.auth.lock(), vec![None]);
}

#[tokio::test]
async fn test_probe_parses_version() {
    let fake = start(default_handler("")).await;
    let version = client(&fake.url).probe("java").await.unwrap();
    assert_eq!(version, "21.0.9");

    let requests = fake.requests.lock();
    let cmd = &requests[0];
    assert_eq!(cmd["args"], json!(["/usr/bin/java", "-version"]));
    assert!(cmd.get("copyIn").is_none());
    assert_eq!(cmd["cpuLimit"], 10_000_000_000u64);
    assert_eq!(cmd["clockLimit"], 15_000_000_000u64);
    assert_eq!(cmd["memoryLimit"], 1_073_741_824u64);
    assert_eq!(cmd["procLimit"], 128);
}

#[tokio::test]
async fn test_probe_failure() {
    let fake = start(results(result("Nonzero Exit Status", 1, "", "no"))).await;
    let err = client(&fake.url).probe("python").await.unwrap_err();
    assert_eq!(err, RunError::InvalidResponse);
}

#[tokio::test]
async fn test_probe_without_version_is_invalid() {
    let fake = start(results(result("Accepted", 0, "hello", ""))).await;
    let err = client(&fake.url).probe("python").await.unwrap_err();
    assert_eq!(err, RunError::InvalidResponse);
}
