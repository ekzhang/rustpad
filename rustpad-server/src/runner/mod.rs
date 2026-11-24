//! Code execution through go-judge.

use std::collections::HashMap;
use std::fmt;
use std::time::Duration;

use log::{info, warn};
use serde::{Deserialize, Serialize};
use tokio::{sync::watch, time};

mod gojudge;
pub(crate) mod languages;

pub use gojudge::GoJudgeClient;

use languages::Language;

/// Wait between runtime list refreshes.
const REFRESH_INTERVAL: Duration = Duration::from_secs(300);

/// First wait after a round in which every probe failed.
const RETRY_INTERVAL: Duration = Duration::from_secs(1);

/// Longest wait after failed rounds.
const MAX_RETRY_INTERVAL: Duration = Duration::from_secs(30);

/// Number of failed probes in a row after which a language is dropped.
const MAX_PROBE_MISSES: u32 = 3;

/// Largest stdout or stderr kept from one stage, in bytes.
pub const MAX_STREAM_BYTES: usize = 64 * 1024;

/// A language this server can run, sent to clients.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Runtime {
    /// Monaco language ID, as used by the editor.
    pub language: String,
    /// Name of the runtime that runs it.
    pub runtime: String,
    /// Runtime version.
    pub version: String,
}

/// Output of one stage (compile or run) of an execution.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct RunStage {
    /// Standard output.
    pub stdout: String,
    /// Standard error.
    pub stderr: String,
    /// Exit code, or `None` if the process was killed by a signal.
    pub code: Option<i32>,
    /// Signal that killed the process, such as `SIGKILL`.
    pub signal: Option<String>,
    /// Why the stage was stopped, such as `Time limit exceeded`.
    pub status: Option<String>,
    /// Set when stdout or stderr was cut to `MAX_STREAM_BYTES`.
    pub truncated: bool,
}

impl RunStage {
    /// Cuts stdout and stderr to `MAX_STREAM_BYTES`, setting `truncated`.
    pub(crate) fn truncate(&mut self) {
        let stdout = truncate_utf8(&mut self.stdout, MAX_STREAM_BYTES);
        let stderr = truncate_utf8(&mut self.stderr, MAX_STREAM_BYTES);
        self.truncated |= stdout || stderr;
    }
}

/// Result of one execution. `run` is absent when compilation fails.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExecOutput {
    /// Compile stage, for compiled languages.
    pub compile: Option<RunStage>,
    /// Run stage, absent if compilation failed.
    pub run: Option<RunStage>,
}

/// Why a runner call failed. `Display` gives a message safe to show users.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RunError {
    /// The runner could not be reached, or answered with an error status.
    Unavailable,
    /// The runner did not answer in time.
    TimedOut,
    /// The runner's answer was too large or not valid JSON.
    InvalidResponse,
}

impl fmt::Display for RunError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            RunError::Unavailable => "Code runner is unavailable",
            RunError::TimedOut => "Code runner timed out",
            RunError::InvalidResponse => "Code runner returned an invalid response",
        })
    }
}

impl std::error::Error for RunError {}

/// Cuts `text` to at most `max` bytes on a character boundary. Returns whether
/// anything was cut.
fn truncate_utf8(text: &mut String, max: usize) -> bool {
    if text.len() <= max {
        return false;
    }
    let mut end = max;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    text.truncate(end);
    true
}

/// Code runner access shared by every document.
#[derive(Clone, Debug)]
pub(crate) struct Runner {
    /// Client used to run code.
    pub(crate) client: GoJudgeClient,
    /// Current list of runnable languages, refreshed in the background.
    pub(crate) runtimes: watch::Receiver<Vec<Runtime>>,
}

impl Runner {
    /// Starts probing languages in the background.
    pub(crate) fn start(client: GoJudgeClient) -> Self {
        Self::start_with_intervals(client, REFRESH_INTERVAL, RETRY_INTERVAL)
    }

    /// Like `start`, with custom waits. `retry` is the first wait after a round
    /// in which every probe failed; it doubles on each miss up to
    /// `MAX_RETRY_INTERVAL`.
    pub(crate) fn start_with_intervals(
        client: GoJudgeClient,
        refresh: Duration,
        retry: Duration,
    ) -> Self {
        let (tx, rx) = watch::channel(Vec::new());
        tokio::spawn(refresher(client.clone(), tx, refresh, retry));
        Self {
            client,
            runtimes: rx,
        }
    }
}

/// Folds one round of probe results into `known` (version and consecutive
/// misses per language) and returns the new runtime list in table order.
///
/// Returns `None`, leaving `known` untouched, when every probe failed.
fn merge(
    known: &mut HashMap<&'static str, (String, u32)>,
    results: &[(&'static Language, Result<String, RunError>)],
) -> Option<Vec<Runtime>> {
    if results.iter().all(|(_, result)| result.is_err()) {
        return None;
    }
    for (lang, result) in results {
        match result {
            Ok(version) => {
                known.insert(lang.id, (version.clone(), 0));
            }
            Err(_) => {
                if let Some((_, misses)) = known.get_mut(lang.id) {
                    *misses += 1;
                    if *misses >= MAX_PROBE_MISSES {
                        known.remove(lang.id);
                    }
                }
            }
        }
    }
    Some(
        languages::LANGUAGES
            .iter()
            .filter_map(|lang| {
                known.get(lang.id).map(|(version, _)| Runtime {
                    language: lang.id.to_string(),
                    runtime: lang.name.to_string(),
                    version: version.clone(),
                })
            })
            .collect(),
    )
}

/// Probes every language now, then every `refresh`. After a round in which
/// every probe fails, retries after `retry`, doubling up to
/// `MAX_RETRY_INTERVAL`.
async fn refresher(
    client: GoJudgeClient,
    tx: watch::Sender<Vec<Runtime>>,
    refresh: Duration,
    retry: Duration,
) {
    let mut known = HashMap::new();
    let mut backoff = retry;
    while !tx.is_closed() {
        let mut results = Vec::with_capacity(languages::LANGUAGES.len());
        for lang in languages::LANGUAGES {
            results.push((lang, client.probe(lang.id).await));
        }
        match merge(&mut known, &results) {
            None => {
                warn!("every language probe failed, retrying in {backoff:?}");
                time::sleep(backoff).await;
                backoff = (backoff * 2).min(MAX_RETRY_INTERVAL);
            }
            Some(runtimes) => {
                tx.send_if_modified(|current| {
                    if *current == runtimes {
                        return false;
                    }
                    info!("Runnable languages: {}", runtimes.len());
                    *current = runtimes;
                    true
                });
                backoff = retry;
                time::sleep(refresh).await;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncate_leaves_short_text() {
        let mut text = String::from("hello");
        assert!(!truncate_utf8(&mut text, 10));
        assert_eq!(text, "hello");
    }

    #[test]
    fn truncate_cuts_long_text() {
        let mut text = "a".repeat(20);
        assert!(truncate_utf8(&mut text, 10));
        assert_eq!(text, "a".repeat(10));
    }

    #[test]
    fn truncate_respects_char_boundary() {
        // "é" is two bytes; a cut at byte 10 would split it.
        let mut text = format!("{}é", "a".repeat(9));
        assert!(truncate_utf8(&mut text, 10));
        assert_eq!(text, "a".repeat(9));
    }

    #[test]
    fn stage_truncate_sets_flag() {
        let mut stage = RunStage {
            stdout: "a".repeat(MAX_STREAM_BYTES + 1),
            stderr: "short".into(),
            ..RunStage::default()
        };
        stage.truncate();
        assert!(stage.truncated);
        assert_eq!(stage.stdout.len(), MAX_STREAM_BYTES);
        assert_eq!(stage.stderr, "short");
    }

    #[test]
    fn stage_serializes_status() {
        assert_eq!(
            serde_json::to_value(RunStage::default()).unwrap(),
            serde_json::json!({"stdout":"","stderr":"","code":null,"signal":null,"status":null,"truncated":false})
        );
    }

    fn lang(id: &str) -> &'static Language {
        languages::find(id).unwrap()
    }

    fn round(
        pairs: &[(&str, Result<&str, RunError>)],
    ) -> Vec<(&'static Language, Result<String, RunError>)> {
        pairs
            .iter()
            .map(|(id, r)| (lang(id), r.clone().map(String::from)))
            .collect()
    }

    fn ids(runtimes: &[Runtime]) -> Vec<&str> {
        runtimes.iter().map(|r| r.language.as_str()).collect()
    }

    #[test]
    fn merge_all_failed_is_none() {
        let mut known = HashMap::new();
        known.insert("python", ("3.13.5".to_string(), 1));
        let results = round(&[
            ("python", Err(RunError::Unavailable)),
            ("c", Err(RunError::TimedOut)),
        ]);
        assert_eq!(merge(&mut known, &results), None);
        assert_eq!(known.len(), 1);
        assert_eq!(known["python"], ("3.13.5".to_string(), 1));
    }

    #[test]
    fn merge_keeps_language_until_third_miss() {
        let mut known = HashMap::new();
        merge(&mut known, &round(&[("python", Ok("3.13.5"))])).unwrap();
        let missed = round(&[
            ("python", Err(RunError::Unavailable)),
            ("javascript", Ok("20.19.2")),
        ]);
        for _ in 0..2 {
            let list = merge(&mut known, &missed).unwrap();
            assert_eq!(ids(&list), ["python", "javascript"]);
        }
        let list = merge(&mut known, &missed).unwrap();
        assert_eq!(ids(&list), ["javascript"]);
    }

    #[test]
    fn merge_resets_misses_on_success() {
        let mut known = HashMap::new();
        let ok = round(&[("python", Ok("3.13.5")), ("c", Ok("14.2.0"))]);
        let missed = round(&[("python", Err(RunError::Unavailable)), ("c", Ok("14.2.0"))]);
        merge(&mut known, &ok).unwrap();
        merge(&mut known, &missed).unwrap();
        merge(&mut known, &missed).unwrap();
        merge(&mut known, &ok).unwrap();
        merge(&mut known, &missed).unwrap();
        let list = merge(&mut known, &missed).unwrap();
        assert_eq!(ids(&list), ["python", "c"]);
    }

    #[test]
    fn merge_lists_in_table_order_with_names() {
        let mut known = HashMap::new();
        let list = merge(
            &mut known,
            &round(&[("cpp", Ok("14.2.0")), ("python", Ok("3.13.5"))]),
        )
        .unwrap();
        assert_eq!(
            list,
            [
                Runtime {
                    language: "python".into(),
                    runtime: "python".into(),
                    version: "3.13.5".into()
                },
                Runtime {
                    language: "cpp".into(),
                    runtime: "g++".into(),
                    version: "14.2.0".into()
                },
            ]
        );
    }

    #[tokio::test]
    async fn refresher_retries_until_runner_answers() {
        use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};
        use std::sync::Arc;
        use warp::{http::StatusCode, Filter};

        // The first round (one probe per language) fails with 500, then python answers.
        let calls = Arc::new(AtomicUsize::new(0));
        let counter = Arc::clone(&calls);
        let route = warp::path("run").and(warp::post()).map(move || {
            if counter.fetch_add(1, AtomicOrdering::SeqCst) < languages::LANGUAGES.len() {
                return warp::reply::with_status("{}".to_string(), StatusCode::INTERNAL_SERVER_ERROR);
            }
            warp::reply::with_status(
                r#"[{"status":"Accepted","exitStatus":0,"files":{"stdout":"Python 3.13.5\n","stderr":""}}]"#
                    .to_string(),
                StatusCode::OK,
            )
        });
        let (addr, server) = warp::serve(route).bind_ephemeral(([127, 0, 0, 1], 0));
        tokio::spawn(server);

        let client = GoJudgeClient::new(&format!("http://{addr}"), None).unwrap();
        // The refresh interval is far beyond the test, so only retries can succeed.
        let runner = Runner::start_with_intervals(
            client,
            Duration::from_secs(3600),
            Duration::from_millis(50),
        );
        let mut runtimes = runner.runtimes.clone();
        time::timeout(Duration::from_secs(2), runtimes.wait_for(|r| !r.is_empty()))
            .await
            .expect("runtimes should appear within 2 s")
            .unwrap();
        assert!(runtimes.borrow().iter().any(|r| r.language == "python"));
    }
}
