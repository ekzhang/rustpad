//! A fake go-judge server for tests.

use std::sync::Arc;
use std::time::Duration;

use parking_lot::Mutex;
use serde_json::{json, Value};
use warp::{http::StatusCode, Filter};

/// How the fake answers `POST /run`.
#[derive(Clone)]
pub enum Reply {
    /// 200 with this JSON array of results.
    Results(Value),
    /// This status with an error body.
    Status(u16),
    /// 200 with this JSON array, after a delay.
    Slow(Duration, Value),
    /// 200 with this raw body.
    Raw(String),
}

/// Decides the answer to one command (`cmd[0]` of the request).
pub type Handler = Arc<dyn Fn(&Value) -> Reply + Send + Sync>;

/// A running fake go-judge.
pub struct FakeJudge {
    /// Base URL, such as `http://127.0.0.1:41234`.
    pub url: String,
    /// `cmd[0]` of each `/run` body, in order.
    pub requests: Arc<Mutex<Vec<Value>>>,
    /// File IDs received on `DELETE /file/{id}`, in order.
    pub deleted: Arc<Mutex<Vec<String>>>,
    /// `Authorization` header of each `/run`, in order.
    pub auth: Arc<Mutex<Vec<Option<String>>>>,
}

/// One go-judge result object.
pub fn result(status: &str, exit: i64, stdout: &str, stderr: &str) -> Value {
    json!({
        "status": status,
        "exitStatus": exit,
        "time": 1_000_000,
        "memory": 1_000_000,
        "runTime": 2_000_000,
        "files": { "stdout": stdout, "stderr": stderr }
    })
}

/// A handler for the usual flow: probes answer with a version, compiles with a
/// cached file, and runs with `stdout`.
pub fn default_handler(stdout: &str) -> Handler {
    let stdout = stdout.to_string();
    Arc::new(move |cmd: &Value| {
        if cmd.get("copyIn").is_none() {
            let program = cmd["args"][0].as_str().unwrap_or("");
            let name = program.rsplit('/').next().unwrap_or("");
            let (out, err) = match name {
                "python3" => ("Python 3.13.5\n", ""),
                "gcc" | "g++" => ("14.2.0\n", ""),
                "java" => ("", "openjdk version \"21.0.9\" 2025-10-21"),
                "kotlinc" => ("info: kotlinc-jvm 2.2.21 (JRE 21.0.9)\n", ""),
                "node" => ("v20.19.2\n", ""),
                _ => ("", ""),
            };
            return Reply::Results(json!([result("Accepted", 0, out, err)]));
        }
        if let Some(cached) = cmd.get("copyOutCached").and_then(Value::as_array) {
            let mut ids = serde_json::Map::new();
            for name in cached.iter().filter_map(Value::as_str) {
                ids.insert(name.trim_end_matches('?').to_string(), json!("FID1"));
            }
            let mut res = result("Accepted", 0, "", "");
            res["fileIds"] = Value::Object(ids);
            return Reply::Results(json!([res]));
        }
        Reply::Results(json!([result("Accepted", 0, &stdout, "")]))
    })
}

/// Starts a fake go-judge on a random local port.
pub async fn start(handler: Handler) -> FakeJudge {
    let requests = Arc::new(Mutex::new(Vec::new()));
    let deleted = Arc::new(Mutex::new(Vec::new()));
    let auth = Arc::new(Mutex::new(Vec::new()));

    let (rec_requests, rec_auth) = (Arc::clone(&requests), Arc::clone(&auth));
    let run_route = warp::path("run")
        .and(warp::path::end())
        .and(warp::post())
        .and(warp::header::optional::<String>("authorization"))
        .and(warp::body::json())
        .then(move |header: Option<String>, body: Value| {
            let cmd = body["cmd"][0].clone();
            rec_auth.lock().push(header);
            let reply = handler(&cmd);
            rec_requests.lock().push(cmd);
            async move {
                match reply {
                    Reply::Results(value) => {
                        warp::reply::with_status(value.to_string(), StatusCode::OK)
                    }
                    Reply::Status(code) => warp::reply::with_status(
                        "fake failure".to_string(),
                        StatusCode::from_u16(code).unwrap(),
                    ),
                    Reply::Slow(delay, value) => {
                        tokio::time::sleep(delay).await;
                        warp::reply::with_status(value.to_string(), StatusCode::OK)
                    }
                    Reply::Raw(body) => warp::reply::with_status(body, StatusCode::OK),
                }
            }
        });

    let rec_deleted = Arc::clone(&deleted);
    let delete_route = warp::path!("file" / String)
        .and(warp::delete())
        .map(move |id: String| {
            rec_deleted.lock().push(id);
            warp::reply::with_status("", StatusCode::OK)
        });

    let (addr, server) =
        warp::serve(run_route.or(delete_route)).bind_ephemeral(([127, 0, 0, 1], 0));
    tokio::spawn(server);
    FakeJudge {
        url: format!("http://{addr}"),
        requests,
        deleted,
        auth,
    }
}
