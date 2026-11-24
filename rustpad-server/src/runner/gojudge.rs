//! Client for the go-judge sandbox REST API.

use std::collections::HashMap;
use std::time::Duration;

use anyhow::Context;
use log::{debug, warn};
use serde::{de::DeserializeOwned, Deserialize};
use serde_json::{json, Map, Value};

use super::languages::{self, Language, Limits, Stage, BASE_ENV, PROBE_LIMITS};
use super::{ExecOutput, RunError, RunStage, MAX_STREAM_BYTES};

/// Largest go-judge response accepted, in bytes.
const MAX_BODY_BYTES: usize = 4 * 1024 * 1024;

/// Time allowed to open a connection to go-judge.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(2);

/// Time allowed for a whole go-judge request.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

/// HTTP client for the two go-judge endpoints Rustpad uses.
#[derive(Clone, Debug)]
pub struct GoJudgeClient {
    http: reqwest::Client,
    base: String,
    token: Option<String>,
}

/// One result of `POST /run`.
#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct JudgeResult {
    status: String,
    exit_status: i64,
    error: Option<String>,
    time: u64,
    memory: u64,
    run_time: u64,
    files: HashMap<String, String>,
    file_ids: HashMap<String, String>,
}

impl GoJudgeClient {
    /// Creates a client with the default timeouts. `token` is sent as a bearer
    /// token.
    pub fn new(base_url: &str, token: Option<String>) -> anyhow::Result<Self> {
        Self::with_timeouts(base_url, token, CONNECT_TIMEOUT, REQUEST_TIMEOUT)
    }

    /// Creates a client with custom timeouts.
    pub fn with_timeouts(
        base_url: &str,
        token: Option<String>,
        connect: Duration,
        total: Duration,
    ) -> anyhow::Result<Self> {
        let base = reqwest::Url::parse(base_url).context("invalid go-judge URL")?;
        let http = reqwest::Client::builder()
            // go-judge is reached directly on the internal network, never through a proxy.
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .connect_timeout(connect)
            .timeout(total)
            .build()
            .context("failed to build HTTP client")?;
        Ok(Self {
            http,
            base: base.as_str().trim_end_matches('/').to_owned(),
            token,
        })
    }

    /// Runs `source`, a program in the Monaco language `language`. An unknown
    /// language gives `Unavailable`.
    pub async fn execute(&self, language: &str, source: &str) -> Result<ExecOutput, RunError> {
        let Some(lang) = languages::find(language) else {
            warn!("no runtime for language {language:?}");
            return Err(RunError::Unavailable);
        };
        let mut cached = Vec::new();
        let output = self.stages(lang, source, &mut cached).await;
        for id in cached {
            self.delete_file(&id).await;
        }
        output
    }

    /// Returns the version of the runtime for `language`.
    pub async fn probe(&self, language: &str) -> Result<String, RunError> {
        let Some(lang) = languages::find(language) else {
            warn!("no runtime for language {language:?}");
            return Err(RunError::Unavailable);
        };
        let cmd = command(
            lang.probe.iter().map(|s| s.to_string()).collect(),
            &[],
            &PROBE_LIMITS,
            None,
            &[],
        );
        let result = self.call(cmd).await?;
        let stdout = result.files.get("stdout").cloned().unwrap_or_default();
        let stderr = result.files.get("stderr").cloned().unwrap_or_default();
        if result.status != "Accepted" {
            warn!("probe of {language} ended with {}", result.status);
            return Err(RunError::InvalidResponse);
        }
        languages::parse_version(&stdout)
            .or_else(|| languages::parse_version(&stderr))
            .ok_or_else(|| {
                warn!("probe of {language} printed no version");
                RunError::InvalidResponse
            })
    }

    /// Compiles if the language needs it, then runs. Pushes the ID of every
    /// cached file into `cached`, so the caller can delete them.
    async fn stages(
        &self,
        lang: &Language,
        source: &str,
        cached: &mut Vec<String>,
    ) -> Result<ExecOutput, RunError> {
        let (file, class) = languages::source_and_class(lang, source);
        let mut copy_in = Map::new();
        copy_in.insert(file.clone(), json!({ "content": source }));

        let mut compile_stage = None;
        let mut run_copy_in = copy_in.clone();
        if let Some(stage) = &lang.compile {
            let cmd = stage_command(stage, &file, &class, copy_in, true);
            let result = self.call(cmd).await?;
            cached.extend(result.file_ids.values().cloned());
            let accepted = result.status == "Accepted";
            let ids = result.file_ids.clone();
            let compiled = to_stage(result)?;
            if !accepted {
                return Ok(ExecOutput {
                    compile: Some(compiled),
                    run: None,
                });
            }
            run_copy_in = Map::new();
            for name in stage.cache {
                let name = name.trim_end_matches('?');
                let Some(id) = ids.get(name) else {
                    warn!("compile did not return {name}");
                    return Err(RunError::InvalidResponse);
                };
                run_copy_in.insert(name.to_string(), json!({ "fileId": id }));
            }
            compile_stage = Some(compiled);
        }

        let cmd = stage_command(&lang.run, &file, &class, run_copy_in, false);
        let run = to_stage(self.call(cmd).await?)?;
        Ok(ExecOutput {
            compile: compile_stage,
            run: Some(run),
        })
    }

    /// Sends one command to `POST /run` and returns its result.
    async fn call(&self, cmd: Value) -> Result<JudgeResult, RunError> {
        let mut request = self
            .http
            .post(format!("{}/run", self.base))
            .json(&json!({ "cmd": [cmd] }));
        if let Some(token) = &self.token {
            request = request.bearer_auth(token);
        }
        let response = request.send().await.map_err(from_reqwest)?;
        let results: Vec<JudgeResult> = parse(response).await?;
        let result = results.into_iter().next().ok_or_else(|| {
            warn!("go-judge returned no result");
            RunError::InvalidResponse
        })?;
        debug!(
            "go-judge status={} time={}ns runTime={}ns memory={}B",
            result.status, result.time, result.run_time, result.memory
        );
        Ok(result)
    }

    /// Deletes a cached file. Failures are logged and ignored.
    async fn delete_file(&self, id: &str) {
        let mut request = self.http.delete(format!("{}/file/{id}", self.base));
        if let Some(token) = &self.token {
            request = request.bearer_auth(token);
        }
        match request.send().await {
            Ok(response) if response.status().is_success() => {}
            Ok(response) => warn!(
                "go-judge refused to delete file {id}: {}",
                response.status()
            ),
            Err(e) => warn!("failed to delete go-judge file {id}: {e}"),
        }
    }
}

/// Builds the command for one stage of a language.
fn stage_command(
    stage: &Stage,
    file: &str,
    class: &str,
    copy_in: Map<String, Value>,
    cache: bool,
) -> Value {
    command(
        languages::expand(stage.args, file, class),
        stage.env,
        &stage.limits,
        Some(copy_in),
        if cache { stage.cache } else { &[] },
    )
}

/// Builds one `cmd` object: stdin empty, stdout and stderr captured.
fn command(
    args: Vec<String>,
    env: &[&str],
    limits: &Limits,
    copy_in: Option<Map<String, Value>>,
    cache: &[&str],
) -> Value {
    let env: Vec<&str> = BASE_ENV.iter().chain(env).copied().collect();
    let mut cmd = json!({
        "args": args,
        "env": env,
        "files": [
            { "content": "" },
            { "name": "stdout", "max": MAX_STREAM_BYTES },
            { "name": "stderr", "max": MAX_STREAM_BYTES },
        ],
        "cpuLimit": limits.cpu_ms * 1_000_000,
        "clockLimit": limits.clock_ms * 1_000_000,
        "memoryLimit": limits.memory_mib * 1024 * 1024,
        "procLimit": limits.procs,
    });
    if let Some(copy_in) = copy_in {
        cmd["copyIn"] = Value::Object(copy_in);
    }
    if !cache.is_empty() {
        cmd["copyOutCached"] = json!(cache);
    }
    cmd
}

/// Maps a go-judge result to a stage.
fn to_stage(result: JudgeResult) -> Result<RunStage, RunError> {
    let mut stage = RunStage {
        stdout: result.files.get("stdout").cloned().unwrap_or_default(),
        stderr: result.files.get("stderr").cloned().unwrap_or_default(),
        ..RunStage::default()
    };
    let exit = i32::try_from(result.exit_status).ok();
    match result.status.as_str() {
        "Accepted" | "Nonzero Exit Status" => stage.code = exit,
        "Output Limit Exceeded" => {
            stage.code = exit;
            stage.truncated = true;
        }
        "Time Limit Exceeded" => {
            stage.signal = Some("SIGKILL".into());
            stage.status = Some("Time limit exceeded".into());
        }
        "Memory Limit Exceeded" => {
            stage.signal = Some("SIGKILL".into());
            stage.status = Some("Memory limit exceeded".into());
        }
        "Signalled" => stage.signal = Some(signal_name(result.exit_status)),
        "Dangerous Syscall" => stage.status = Some("Blocked system call".into()),
        other => {
            warn!(
                "go-judge status {other}: {}",
                result.error.as_deref().unwrap_or("no error message")
            );
            return Err(RunError::InvalidResponse);
        }
    }
    stage.truncate();
    Ok(stage)
}

/// Names a signal number.
fn signal_name(n: i64) -> String {
    match n {
        4 => "SIGILL",
        6 => "SIGABRT",
        7 => "SIGBUS",
        8 => "SIGFPE",
        9 => "SIGKILL",
        11 => "SIGSEGV",
        13 => "SIGPIPE",
        15 => "SIGTERM",
        24 => "SIGXCPU",
        25 => "SIGXFSZ",
        other => return format!("signal {other}"),
    }
    .to_string()
}

/// Reads a capped response body and decodes it as JSON.
async fn parse<T: DeserializeOwned>(mut response: reqwest::Response) -> Result<T, RunError> {
    let status = response.status();
    let mut body = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(from_reqwest)? {
        if body.len() + chunk.len() > MAX_BODY_BYTES {
            warn!("go-judge response exceeded {MAX_BODY_BYTES} bytes");
            return Err(RunError::InvalidResponse);
        }
        body.extend_from_slice(&chunk);
    }
    if !status.is_success() {
        warn!(
            "go-judge returned {status}: {}",
            String::from_utf8_lossy(&body)
        );
        return Err(RunError::Unavailable);
    }
    serde_json::from_slice(&body).map_err(|e| {
        warn!("invalid go-judge response: {e}");
        RunError::InvalidResponse
    })
}

/// Logs a transport error and maps it to a user-facing one.
fn from_reqwest(e: reqwest::Error) -> RunError {
    warn!("go-judge request failed: {e}");
    if e.is_timeout() {
        RunError::TimedOut
    } else {
        RunError::Unavailable
    }
}
