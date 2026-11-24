# Code execution with go-judge Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let people in a pad run its code (Python, C, C++, Java, Kotlin, JavaScript) through a self-hosted go-judge runner, with the Run button and shared output panel from `feature/execution-runtime`.

**Architecture:** The Rustpad server owns a fixed language table and talks to go-judge's REST API (`POST /run`, `DELETE /file/{id}`). Compiled languages run in two go-judge calls: compile with `copyOutCached`, then run by `fileId`. A background refresher probes each language's version through go-judge and broadcasts the runnable list. The runner image (go-judge plus Debian compilers) lives in `runner/` and is built by `podman compose`.

**Tech Stack:** Rust (warp, tokio, reqwest with rustls, serde), TypeScript/React/Chakra, go-judge v1.14.1, Debian 13, Podman compose.

**Spec:** `docs/superpowers/specs/2026-11-19-code-execution-go-judge-design.md`. The Piston spec it builds on is `docs/superpowers/specs/2026-10-09-code-execution-design.md` on `feature/execution-runtime` (read it with `git show feature/execution-runtime:docs/superpowers/specs/2026-10-09-code-execution-design.md`).

## Global Constraints

- Branch: `feature/execution-go-judge` (already created off `main`). No Piston code, names, env vars or docs may remain at the end.
- Reused code comes from `feature/execution-runtime`; fetch it with `git show feature/execution-runtime:<path>` or `git checkout feature/execution-runtime -- <path>`.
- Commits: one subject line, no body, no co-author line. Prefix with the Jira ticket ID if the user gave one. Never `git push`.
- Never stage `.env`. Git shows it as deleted because the sandbox blocks reading it. Always `git add` explicit paths, never `git add -A` or `git add .`.
- Wire protocol is unchanged from the Piston branch except `RunStage` gains `status: string | null`.
- Env vars: `GO_JUDGE_URL` (unset = code execution off), `GO_JUDGE_TOKEN` (optional bearer token).
- Fixed user-facing errors: "Code runner is unavailable", "Code runner timed out", "Code runner returned an invalid response", "No runtime for <language>", "Code too large to run (64 KiB limit)".
- Limits: code ≤ 64 KiB; stdout and stderr each cut to 64 KiB on a UTF-8 boundary; response body cap 4 MiB; connect timeout 2 s; request timeout 30 s.
- Status strings: "Time limit exceeded", "Memory limit exceeded", "Blocked system call".
- A language that passed its probe stays listed until it fails 3 refreshes in a row; a round where every probe fails changes nothing and retries with backoff (1 s doubling to 30 s). Refresh interval 5 min.
- Output renders only as React text children (no `dangerouslySetInnerHTML`, no ANSI-to-HTML).
- Runner image: `FROM criyle/go-judge` pinned by multi-arch digest; Debian packages with `--no-install-recommends`; no architecture-specific steps.
- Docs follow `~/Scripts/personal/claude-common/10-documentation.md`: describe the current state only, never "instead of Piston".
- Frontend has no test runner; `npm run check` must pass.

## Review Focus

1. Two pads running at the same time: both get their own results, with no crosstalk between their go-judge calls. Test: `test_two_pads_run_concurrently` in Task 6.
2. go-judge restarts between compile and run, so the cached `fileId` is gone: the run fails with "Code runner returned an invalid response", the `DELETE` is still attempted, and the next run works. Test: `test_execute_run_file_error_still_deletes` in Task 4.
3. A program that reads stdin (`input()`) gets EOF at once instead of hanging until the time limit. Test: `test_shared_run` in Task 6 asserts `files[0] == {"content": ""}`.
4. Multi-byte output (emoji) at the 64 KiB cut yields valid text and `truncated: true`, never a panic. Test: `test_execute_truncates_multibyte_output` in Task 4.
5. Java code with a `package` line and a public class not named `Main` compiles and runs. Test: `java_entry_handles_package` in Task 3, and the `Solution` program in the smoke test (Task 8).

---

## File Structure

| File | Responsibility |
|---|---|
| `build.sh`, `Dockerfile`, `.dockerignore`, `.gitignore`, `.github/workflows/ci.yml` | Host-built server binary and an image that only packages it (from the Piston branch) |
| `runner/Dockerfile` | Runner image: go-judge, compilers, Kotlin |
| `runner/mount.yaml` | go-judge sandbox mounts for these languages |
| `compose/compose.yml` | Local stack: `runner` and `rustpad` |
| `compose/smoke-test.sh` | Runs the ignored smoke tests against the local runner |
| `rustpad-server/src/runner/mod.rs` | Shared types (`Runtime`, `RunStage`, `ExecOutput`, `RunError`), truncation, `Runner` and its refresher |
| `rustpad-server/src/runner/languages.rs` | Language table, limits, version parsing, Java entry naming |
| `rustpad-server/src/runner/gojudge.rs` | `GoJudgeClient`: HTTP, request building, result mapping, `execute`, `probe` |
| `rustpad-server/src/rustpad.rs`, `lib.rs`, `main.rs` | Run flow and config wiring (ported from the Piston branch) |
| `rustpad-server/tests/common/go_judge.rs` | Fake go-judge for tests |
| `rustpad-server/tests/judge_client.rs` | Client tests against the fake |
| `rustpad-server/tests/run.rs` | End-to-end run tests over WebSocket |
| `rustpad-server/tests/smoke.rs` | `#[ignore]` tests against a real runner |
| `src/rustpad.ts`, `src/App.tsx`, `src/Sidebar.tsx`, `src/RunOutput.tsx` | Frontend (from the Piston branch, plus `status`) |
| `README.md` | Build, configuration, code execution docs |

---

### Task 1: Bring over the host build

**Files:**
- Copy from `feature/execution-runtime`: `build.sh`, `Dockerfile`, `.dockerignore`, `.gitignore`, `.github/workflows/ci.yml`, `README.md`
- Modify: `README.md`

**Interfaces:**
- Produces: `./build.sh` builds `dist/` and `build/linux-<arch>/rustpad-server`; root `Dockerfile` packages them.

- [ ] **Step 1: Copy the files**

Run: `git checkout feature/execution-runtime -- build.sh Dockerfile .dockerignore .gitignore .github/workflows/ci.yml README.md`

- [ ] **Step 2: Strip Piston from the README**

Delete the `PISTON_URL` bullet under "Configuration" and the whole "Code execution" section with its subsections (through the end of "Piston in production"). Task 8 writes the new section. Keep the rest, including the fork note at the top and the build instructions.

- [ ] **Step 3: Verify the build**

Run: `./build.sh`
Expected: exits 0, and `build/linux-arm64/rustpad-server` exists (`ls build/linux-arm64`).

- [ ] **Step 4: Commit**

```bash
git add build.sh Dockerfile .dockerignore .gitignore .github/workflows/ci.yml README.md
git commit -m "Build the server binary on the host and only package it in the image"
```

---

### Task 2: Runner image

**Files:**
- Create: `runner/Dockerfile`, `runner/mount.yaml`, `compose/compose.yml`

**Interfaces:**
- Produces: image `localhost/rustpad-runner:dev` serving go-judge on `:5050`. Binaries at `/usr/bin/python3`, `/usr/bin/gcc`, `/usr/bin/g++`, `/usr/bin/javac`, `/usr/bin/jar`, `/usr/bin/java`, `/usr/bin/node`, `/usr/local/bin/kotlinc`, `/bin/sh`. Everything outside `/usr`, `/bin`, `/lib*` and the `/etc` paths in `mount.yaml` is invisible inside the sandbox, so Kotlin must live under `/usr/local`.

- [ ] **Step 1: Find the pins**

Run: `podman manifest inspect docker.io/criyle/go-judge:v1.14.1` (fall back to `:latest` if that tag is missing). Take the manifest-list digest for `FROM ...@sha256:...`. Pick the latest stable Kotlin release from `https://github.com/JetBrains/kotlin/releases` and note its `kotlin-compiler-<v>.zip` SHA-256 (from the release's `.sha256` asset).

- [ ] **Step 2: Write `runner/Dockerfile`**

- Stage `kotlin`: `debian:trixie-slim`, install `curl ca-certificates unzip`, download the zip, check it with `sha256sum -c`, unzip to `/kotlinc`. `ARG KOTLIN_VERSION`, `ARG KOTLIN_SHA256`.
- Final stage: `FROM docker.io/criyle/go-judge:v1.14.1@sha256:<digest>`. `apt-get install -y --no-install-recommends python3 gcc g++ libc6-dev openjdk-21-jdk-headless nodejs`, then clean the apt lists. `COPY --from=kotlin /kotlinc /usr/local/lib/kotlinc` and symlink `/usr/local/bin/kotlinc`. `COPY mount.yaml /opt/mount.yaml`.
- `CMD ["/opt/go-judge", "-http-addr", ":5050", "-file-timeout", "10m"]`. Check the token flag with `podman run --rm <image> /opt/go-judge -h`, and document in a Dockerfile comment the env var that sets `-auth-token` (go-judge reads flags from `ES_`-prefixed env vars; confirm the exact name).

- [ ] **Step 3: Write `runner/mount.yaml`**

Start from the stock `/opt/mount.yaml` (`podman run --rm --entrypoint cat docker.io/criyle/go-judge /opt/mount.yaml`). Drop the Pascal, Mono and GHC binds, `containerPasswd.txt` and `dotenv`. Keep the `/bin`, `/lib`, `/lib64`, `/usr`, `/etc/ld.so.cache`, `/etc/alternatives` and `/etc/java-21-openjdk` binds, the `/dev/null`, `/dev/urandom`, `/dev/random` and `/dev/zero` binds, the `/w` and `/tmp` tmpfs mounts (128m each), `proc: true`, the symlinks, `maskPath`, `workDir: /w` and uid/gid 1536.

- [ ] **Step 4: Write `compose/compose.yml` with the `runner` service only**

`build: ../runner`, `image: localhost/rustpad-runner:dev`, `privileged: true` with the comment `# go-judge creates namespaces and cgroups for every run.`, `shm_size: 256m`, `ports: ["127.0.0.1:5050:5050"]`. Task 7 adds `rustpad`.

- [ ] **Step 5: Build and check the tools**

Run (with `REGISTRY_AUTH_FILE` pointing at a `{}` file inside the Claude sandbox):
`podman compose -f compose/compose.yml build runner && podman run --rm --entrypoint sh localhost/rustpad-runner:dev -c 'python3 --version; gcc -dumpfullversion; g++ -dumpfullversion; java -version; kotlinc -version; node --version'`
Expected: six version lines, no errors. Record `podman image ls localhost/rustpad-runner:dev` size for Task 8.

- [ ] **Step 6: Commit**

```bash
git add runner/Dockerfile runner/mount.yaml compose/compose.yml
git commit -m "Add go-judge runner image and compose service"
```

---

### Task 3: Runner types and language table

**Files:**
- Create: `rustpad-server/src/runner/mod.rs`, `rustpad-server/src/runner/languages.rs`
- Modify: `rustpad-server/src/lib.rs` (add `pub mod runner;`), `rustpad-server/Cargo.toml` (add `reqwest = { version = "0.11", default-features = false, features = ["json", "rustls-tls"] }`, as on the Piston branch)

**Interfaces:**
- Produces in `runner/mod.rs` (port from `piston.rs` on the Piston branch, minus everything Piston-specific):
  - `pub const MAX_STREAM_BYTES: usize = 64 * 1024;`
  - `pub struct Runtime { pub language: String, pub runtime: String, pub version: String }` (Serialize, Deserialize, Clone, PartialEq, Eq, Debug)
  - `pub struct RunStage { pub stdout: String, pub stderr: String, pub code: Option<i32>, pub signal: Option<String>, pub status: Option<String>, pub truncated: bool }` with `fn truncate(&mut self)`
  - `pub struct ExecOutput { pub compile: Option<RunStage>, pub run: Option<RunStage> }`
  - `pub enum RunError { Unavailable, TimedOut, InvalidResponse }` with the same `Display` messages
  - `fn truncate_utf8(text: &mut String, max: usize) -> bool`
- Produces in `runner/languages.rs` (all `pub(crate)`):
  - `struct Limits { cpu_ms: u64, clock_ms: u64, memory_mib: u64, procs: u64 }`
  - `struct Stage { args: &'static [&'static str], env: &'static [&'static str], limits: Limits, cache: &'static [&'static str] }`. `args` may hold the whole-token placeholders `{file}` and `{class}`. `env` is added to `BASE_ENV`. `cache` lists `copyOutCached` names.
  - `enum SourceName { Fixed(&'static str), JavaClass }`
  - `struct Language { id: &'static str, name: &'static str, source: SourceName, compile: Option<Stage>, run: Stage, probe: &'static [&'static str] }`
  - `const BASE_ENV: &[&str] = &["PATH=/usr/local/bin:/usr/bin:/bin", "HOME=/w", "LANG=C.UTF-8"];`
  - `const PROBE_LIMITS: Limits` = 10 000 ms CPU, 15 000 ms clock, 512 MiB, 128 procs
  - `const LANGUAGES: &[Language]`, `fn find(id: &str) -> Option<&'static Language>`
  - `fn parse_version(output: &str) -> Option<String>`: the first `\d+(\.\d+)+` match, hand-written (no regex crate)
  - `struct JavaEntry { file: String, class: String }`, `fn java_entry(source: &str) -> JavaEntry`
  - `fn source_and_class(lang: &Language, source: &str) -> (String, String)`: the file name and class for placeholders
  - `fn expand(args: &[&str], file: &str, class: &str) -> Vec<String>`

Table values (starting points; Task 8 calibrates):

| id | name | source | compile args | compile limits | cache | run args | run limits | probe |
|---|---|---|---|---|---|---|---|---|
| `python` | `python` | `main.py` | – | – | – | `/usr/bin/python3 main.py` | 3000/5000/256/64 | `/usr/bin/python3 --version` |
| `c` | `gcc` | `main.c` | `/usr/bin/gcc -O2 -std=c17 main.c -o main -lm` | 10000/15000/512/64 | `main?` | `main` | 3000/5000/256/64 | `/usr/bin/gcc -dumpfullversion` |
| `cpp` | `g++` | `main.cpp` | `/usr/bin/g++ -O2 -std=c++20 main.cpp -o main` | 10000/15000/512/64 | `main?` | `main` | 3000/5000/256/64 | `/usr/bin/g++ -dumpfullversion` |
| `java` | `java` | JavaClass | `/bin/sh -c 'javac -J-Xmx384m -J-XX:+UseSerialGC -J-XX:TieredStopAtLevel=1 -d classes "$0" && jar -J-Djava.io.tmpdir=. --create --file main.jar --main-class "$1" -C classes .' {file} {class}` | 20000/15000/512/128 | `main.jar?` | `/usr/bin/java -Xmx192m -XX:+UseSerialGC -XX:TieredStopAtLevel=1 -jar main.jar` | 5000/5000/256/128 | `/usr/bin/java -version` |
| `kotlin` | `kotlin` | `main.kt` | `/usr/local/bin/kotlinc -J-Xmx384m -J-XX:+UseSerialGC main.kt -include-runtime -d main.jar` | 20000/15000/512/128 | `main.jar?` | same as Java run | 5000/5000/256/128 | `/usr/local/bin/kotlinc -version` |
| `javascript` | `node` | `main.js` | – | – | – | `/usr/bin/node main.js` | 3000/5000/256/64 | `/usr/bin/node --version` |

`jar` needs `-J-Djava.io.tmpdir=.`: by default it writes its temp file to `/tmp` and copies it into `/w`, a different tmpfs, and the sandbox refuses that copy (`IOException: Operation not permitted` in `directCopy0`). Measured on the real image: Java compile 0.25 s wall, run 20 ms; Kotlin compile 2.6 s wall, 4.2 s CPU, 416 MiB; `kotlinc -version` 1.2 s wall, 314 MiB.

Limits read CPU ms / clock ms / MiB / procs. A trailing `?` marks a `copyOutCached` name as optional (go-judge `copyOutOptional`), so a failed compile is not a `File Error`.

`java_entry` rules: the class is the first `public` top-level class (optionally `final` or `abstract`), else the first `class`, else `Main`. The file is `<simple name>.java`. If the source has `package a.b;`, the class becomes `a.b.<Name>`. Only `[A-Za-z0-9_$]` identifiers are accepted; anything else falls back to `Main`. Comments and strings are not parsed.

- [ ] **Step 1: Write the failing unit tests**

In `languages.rs`:
- `parse_version_cases`: `"Python 3.13.5\n"` → `"3.13.5"`; `"14.2.0\n"` → `"14.2.0"`; `"openjdk version \"21.0.9\" 2025-10-21"` → `"21.0.9"`; `"info: kotlinc-jvm 2.2.21 (JRE 21.0.9+10)"` → `"2.2.21"`; `"v20.19.2\n"` → `"20.19.2"`; `"no digits"` → `None`; `"version 7"` → `None`.
- `java_entry_public_class`: `"public class Solution { public static void main(String[] a) {} }"` → file `Solution.java`, class `Solution`.
- `java_entry_without_public_class`: `"class Foo {}\nclass Bar {}"` → `Foo.java`, `Foo`.
- `java_entry_public_final`: `"public final class App {}"` → `App.java`, `App`.
- `java_entry_defaults_to_main`: `"interface X {}"` → `Main.java`, `Main`.
- `java_entry_handles_package`: `"package demo.app;\npublic class Solution {}"` → `Solution.java`, `demo.app.Solution`.
- `expand_replaces_placeholders`: `expand(&["sh", "{file}", "{class}", "x{file}"], "A.java", "A")` → `["sh", "A.java", "A", "x{file}"]` (whole tokens only).
- `table_ids_are_editor_languages`: every `LANGUAGES[i].id` is in `src/languages.json` (load it with `include_str!("../../../src/languages.json")`), and the ids are exactly `python, c, cpp, java, kotlin, javascript`.
- `find_unknown_is_none`: `find("markdown").is_none()`.

In `mod.rs`: port `truncate_leaves_short_text`, `truncate_cuts_long_text`, `truncate_respects_char_boundary`, and `stage_truncate_sets_flag` from `piston.rs`. Add `stage_serializes_status`: `serde_json::to_value(RunStage::default())` equals `{"stdout":"","stderr":"","code":null,"signal":null,"status":null,"truncated":false}`.

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test -p rustpad-server --lib runner`
Expected: compile errors for the missing items.

- [ ] **Step 3: Implement the types, table and helpers**

- [ ] **Step 4: Run the tests**

Run: `cargo test -p rustpad-server --lib runner`
Expected: all pass.

- [ ] **Step 5: Commit**

```bash
git add rustpad-server/Cargo.toml Cargo.lock rustpad-server/src/lib.rs rustpad-server/src/runner
git commit -m "Add runner types and language table"
```

---

### Task 4: go-judge client

**Files:**
- Create: `rustpad-server/src/runner/gojudge.rs`, `rustpad-server/tests/common/go_judge.rs`, `rustpad-server/tests/judge_client.rs`
- Modify: `rustpad-server/src/runner/mod.rs` (`mod gojudge; pub use gojudge::GoJudgeClient;`), `rustpad-server/tests/common/mod.rs` (`pub mod go_judge;`)

**Interfaces:**
- Consumes: Task 3 types and table.
- Produces:
  - `GoJudgeClient::new(base_url: &str, token: Option<String>) -> anyhow::Result<Self>`
  - `GoJudgeClient::with_timeouts(base_url: &str, token: Option<String>, connect: Duration, total: Duration) -> anyhow::Result<Self>`
  - `pub async fn execute(&self, language: &str, source: &str) -> Result<ExecOutput, RunError>`. `language` is a Monaco ID; an unknown ID logs a warning and returns `Unavailable`.
  - `pub async fn probe(&self, language: &str) -> Result<String, RunError>`. It returns the parsed version. Probe failure means a transport error, a status other than `Accepted`, or no version found in stdout followed by stderr; the last two map to `InvalidResponse`.
  - Fake: `tests/common/go_judge.rs` exposing `FakeJudge { url: String, requests: Arc<Mutex<Vec<Value>>>, deleted: Arc<Mutex<Vec<String>>>, auth: Arc<Mutex<Vec<Option<String>>>> }`, `enum Reply { Results(Value), Status(u16), Slow(Duration, Value), Raw(String) }`, `type Handler = Arc<dyn Fn(&Value) -> Reply + Send + Sync>`, `fn result(status: &str, exit: i64, stdout: &str, stderr: &str) -> Value` (one go-judge result object), `fn default_handler(stdout: &str) -> Handler`, `async fn start(handler: Handler) -> FakeJudge`. `requests` records each body's `cmd[0]`; `deleted` records `DELETE /file/{id}` ids; `auth` records the `Authorization` header of each `/run`.
  - `default_handler`: a cmd without `copyIn` is a probe and answers `Accepted` with a version by `args[0]` basename (`python3` → `Python 3.13.5\n`; `gcc`, `g++` → `14.2.0\n`; `java` → stderr `openjdk version "21.0.9" 2025-10-21`; `kotlinc` → `info: kotlinc-jvm 2.2.21 (JRE 21.0.9)\n`; `node` → `v20.19.2\n`). A cmd with `copyOutCached` is a compile and answers `Accepted` with `fileIds` `{"main": "FID1"}` or `{"main.jar": "FID1"}` (the name without `?`). Anything else answers `Accepted` with `stdout`.

Request body per call: `{"cmd":[{ "args", "env": BASE_ENV + stage.env, "files": [{"content": ""}, {"name": "stdout", "max": 65536}, {"name": "stderr", "max": 65536}], "cpuLimit": ns, "clockLimit": ns, "memoryLimit": bytes, "procLimit", "copyIn": {name: {"content": src}} or {name: {"fileId": id}}, "copyOutCached"? }]}`. Send `Authorization: Bearer <token>` when a token is set. Build the HTTP client as `PistonClient::with_timeouts` did: `no_proxy`, no redirects, and the same capped body reader with logging.

Result mapping (`fn to_stage(result: JudgeResult) -> Result<RunStage, RunError>`; JudgeResult has `status`, `exitStatus`, `error`, `time`, `memory`, `runTime`, `files`, `fileIds`):

| go-judge status | code | signal | status | truncated |
|---|---|---|---|---|
| `Accepted`, `Nonzero Exit Status` | `exitStatus` | – | – | from cut |
| `Output Limit Exceeded` | `exitStatus` | – | – | `true` |
| `Time Limit Exceeded` | – | `SIGKILL` | Time limit exceeded | from cut |
| `Memory Limit Exceeded` | – | `SIGKILL` | Memory limit exceeded | from cut |
| `Signalled` | – | name of `exitStatus` | – | from cut |
| `Dangerous Syscall` | – | – | Blocked system call | from cut |
| `Internal Error`, `File Error`, other | `Err(InvalidResponse)`, logged with `error` | | | |

Signal names: 4 SIGILL, 6 SIGABRT, 7 SIGBUS, 8 SIGFPE, 9 SIGKILL, 11 SIGSEGV, 13 SIGPIPE, 15 SIGTERM, 24 SIGXCPU, 25 SIGXFSZ, anything else `signal <n>`. Log each call at `debug!` with status, `time`, `runTime` and `memory`; Task 8 uses those lines to calibrate.

`execute` flow: compile when the language has a compile stage. On a non-`Accepted` compile, return `ExecOutput { compile: Some(stage), run: None }`. On `Accepted` without every cache `fileId`, return `InvalidResponse`. Otherwise run with the cached files, then `DELETE` every `fileId`, whatever the run returned (`DELETE` errors are logged and ignored). The compile stage is returned as `Some` even when it succeeded. Interpreted languages make one call with `compile: None`.

- [ ] **Step 1: Write the fake and the failing client tests**

In `tests/judge_client.rs` (port the shape of `tests/piston_client.rs` from the Piston branch):
- `test_execute_python_request`: the request has `args == ["/usr/bin/python3", "main.py"]`, `copyIn["main.py"]["content"] == "print(1)"`, `cpuLimit == 3_000_000_000`, `clockLimit == 5_000_000_000`, `memoryLimit == 268435456`, `procLimit == 64`, `files[0] == {"content": ""}`, `files[1] == {"name": "stdout", "max": 65536}`, `env` contains `"HOME=/w"`, no `copyOutCached`; the result `run.stdout` is the fake's, and `compile` is `None`.
- `test_execute_cpp_two_steps`: three events in order: compile request (`copyOutCached == ["main?"]`, `copyIn["main.cpp"]`), run request with `copyIn["main"] == {"fileId": "FID1"}` and `args == ["main"]`, then `deleted == ["FID1"]`; `compile` is `Some` with `code == Some(0)`.
- `test_execute_java_names_file_after_class`: source `public class Solution {...}`; compile `args` end with `["Solution.java", "Solution"]`, and `copyIn` has `Solution.java`.
- `test_execute_compile_error_skips_run`: compile answers `Nonzero Exit Status`, exit 1, stderr `error`; result `run` is `None`; one request; `deleted` empty.
- `test_execute_maps_statuses`: one case per row of the mapping table, including `Signalled` 11 → `SIGSEGV` and `Signalled` 42 → `signal 42`.
- `test_execute_internal_error`: `Internal Error` → `Err(RunError::InvalidResponse)`.
- `test_execute_run_file_error_still_deletes`: cpp compile OK, run answers `File Error` → `Err(InvalidResponse)` and `deleted == ["FID1"]`.
- `test_execute_truncates_output`: stdout of `MAX_STREAM_BYTES + 10` `a` → length `MAX_STREAM_BYTES`, `truncated`.
- `test_execute_truncates_multibyte_output`: stdout of `"😀".repeat(MAX_STREAM_BYTES / 4 + 1)` → valid string, `len() <= MAX_STREAM_BYTES`, `len() % 4 == 0`, `truncated`.
- `test_execute_server_error` (500 → `Unavailable`), `test_execute_timeout` (`Slow` past a 300 ms total timeout → `TimedOut`), `test_execute_oversized_body` (over 4 MiB → `InvalidResponse`), `test_execute_invalid_json` (→ `InvalidResponse`), `test_connection_refused` (→ `Unavailable`), `test_rejects_invalid_url`.
- `test_sends_bearer_token`: a client with `Some("secret")` → `auth == [Some("Bearer secret")]`; without one → `[None]`.
- `test_probe_parses_version`: `probe("java")` → `"21.0.9"` (the version from stderr); the request has no `copyIn` and uses `PROBE_LIMITS`.
- `test_probe_failure`: a probe answering `Nonzero Exit Status` → `Err(InvalidResponse)`.

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test -p rustpad-server --test judge_client`
Expected: compile errors (no `GoJudgeClient`).

- [ ] **Step 3: Implement `gojudge.rs`**

- [ ] **Step 4: Run the tests**

Run: `cargo test -p rustpad-server --test judge_client && cargo test -p rustpad-server --lib runner`
Expected: all pass.

- [ ] **Step 5: Commit**

```bash
git add rustpad-server/src/runner rustpad-server/tests/common/mod.rs rustpad-server/tests/common/go_judge.rs rustpad-server/tests/judge_client.rs
git commit -m "Add go-judge client"
```

---

### Task 5: Runtime list and configuration

**Files:**
- Modify: `rustpad-server/src/runner/mod.rs` (add `Runner`, `refresher`, `merge`), `rustpad-server/src/lib.rs`, `rustpad-server/src/main.rs`, `rustpad-server/src/rustpad.rs`, `rustpad-server/tests/persist.rs`
- Create: `rustpad-server/tests/run.rs` (first tests only)

**Interfaces:**
- Consumes: `GoJudgeClient::probe`, `languages::LANGUAGES`.
- Produces:
  - `pub(crate) struct Runner { pub(crate) client: GoJudgeClient, pub(crate) runtimes: watch::Receiver<Vec<Runtime>> }` with `start(client)` and `start_with_intervals(client, refresh, retry)`, as on the Piston branch.
  - `const MAX_PROBE_MISSES: u32 = 3;`
  - `fn merge(known: &mut HashMap<&'static str, (String, u32)>, results: &[(&'static Language, Result<String, RunError>)]) -> Option<Vec<Runtime>>`. It returns `None` when every result is `Err`, leaving `known` untouched. Otherwise `Ok` resets that language to `(version, 0)`, and `Err` adds a miss to a known language, dropping it at `MAX_PROBE_MISSES`. The list comes back in table order, with `runtime` set to `Language::name`.
  - `refresher`: probes every language in table order each round. `None` from `merge` means: log, sleep the backoff (doubling up to 30 s), retry. Otherwise `send_if_modified` and sleep `refresh`.
  - `ServerConfig.runner: Option<GoJudgeClient>` (default `None`); `ServerState.runner: Option<Runner>`.
  - `main.rs`: `GO_JUDGE_URL` → `GoJudgeClient::new(&url, std::env::var("GO_JUDGE_TOKEN").ok())`, panicking with "Unable to parse GO_JUDGE_URL" on a bad URL.
  - `rustpad.rs`: port the runtime parts of the Piston branch's `rustpad.rs`: `with_runner`, `runtimes_rx` in `handle_connection`, `Runtimes` in `send_initial`, `next_runtimes`, and the `ServerMsg::Runtimes` variant. The run flow waits for Task 6.

- [ ] **Step 1: Write the failing tests**

Unit tests in `runner/mod.rs`:
- `merge_all_failed_is_none`: with `known` holding python, all `Err` → `None`, and `known` is unchanged.
- `merge_keeps_language_until_third_miss`: python `Ok("3.13.5")` then three rounds with python `Err` and node `Ok`: after rounds 1 and 2 the list holds python; after round 3 it doesn't.
- `merge_resets_misses_on_success`: two misses, an `Ok`, then two more misses → still listed.
- `merge_lists_in_table_order_with_names`: `Ok` for `cpp` and `python` → `[python/python, cpp/g++]`.
- `refresher_retries_until_runner_answers`: port `refresher_retries_until_runtimes_appear`, using an inline warp `/run` route that returns 500 for the first 6 calls (one full round), then `Accepted` with `Python 3.13.5`; runtimes appear within 2 s.

Integration tests in `tests/run.rs` (port the helpers `config`, `expect_runtimes`, `open_pad`, `join_pad` from the Piston branch's `tests/run.rs`, with `config` building `GoJudgeClient::with_timeouts(&fake.url, None, 1 s, 300 ms)`):
- `test_runtimes_on_connect`: the list contains `{"language":"python","runtime":"python","version":"3.13.5"}`, `{"language":"cpp","runtime":"g++","version":"14.2.0"}` and `{"language":"kotlin","runtime":"kotlin","version":"2.2.21"}`, and no `markdown`.
- `test_no_runtimes_when_off`: port as is.
- `test_failed_probe_hides_language`: the handler fails the `kotlinc` probe → the list has python and no kotlin.

Fix `tests/persist.rs` to use `..ServerConfig::default()` as the Piston branch did.

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test -p rustpad-server`
Expected: compile errors for `Runner`, `merge` and `ServerConfig.runner`.

- [ ] **Step 3: Implement it**

- [ ] **Step 4: Run all server tests**

Run: `cargo test -p rustpad-server`
Expected: all pass, including the existing tests.

- [ ] **Step 5: Commit**

```bash
git add rustpad-server/src rustpad-server/tests/run.rs rustpad-server/tests/persist.rs
git commit -m "Send runnable languages to clients"
```

---

### Task 6: Run flow

**Files:**
- Modify: `rustpad-server/src/rustpad.rs`, `rustpad-server/tests/run.rs`

**Interfaces:**
- Consumes: `Runner`, `GoJudgeClient::execute(language, source)`.
- Produces: `ClientMsg::Run`, `ServerMsg::{RunStarted, RunFinished, RunFailed}`, `RunState`, `start_run`, the replay in `send_initial`, `MAX_RUN_CODE_BYTES`, all ported from the Piston branch's `rustpad.rs`. The only change is the call `client.execute(&runtime.language, &text)`. Update the `MAX_RUN_CODE_BYTES` comment, which mentions Piston.

- [ ] **Step 1: Port the run tests**

Port these from the Piston branch's `tests/run.rs`, swapping the fake for `go_judge::start(...)` and the expected values for go-judge ones: `test_shared_run`, `test_compile_error`, `test_busy_run_ignored`, `test_language_change_during_run`, `test_unsupported_language`, `test_code_too_large`, `test_run_without_language`, `test_piston_error` → `test_runner_error`, `test_piston_timeout` → `test_runner_timeout`, `test_late_joiner_sees_last_result`, `test_late_joiner_sees_run_in_progress`, `test_runner_leaves_mid_run`, `test_run_ignored_when_off`. Changed assertions:
- `test_shared_run`: `runtime == "python"`, `version == "3.13.5"`, `run == {"stdout":"hi\n","stderr":"","code":0,"signal":null,"status":null,"truncated":false}`. The fake's last request has `copyIn["main.py"]["content"] == "print('hi')"` and `files[0] == {"content": ""}`.
- `test_compile_error`: language `cpp`, `runtime == "g++"`, `compile.stderr == "error"`, `run == null`, and the fake saw one request.
- `test_runner_error`: 500 → `RunFailed` with "Code runner is unavailable".
- `test_runner_timeout`: → "Code runner timed out".

Add:
- `test_compiled_run_finishes`: cpp → `RunFinished` with `compile.code == 0` and `run.stdout` from the fake; `deleted == ["FID1"]`.
- `test_two_pads_run_concurrently`: pads `a` (python) and `b` (javascript) both send `Run` against a `Slow(150 ms)` handler. Each pad gets its own `RunStarted { id: 0 }` and a `RunFinished` with its own `language`, and the fake saw one run request with `main.py` and one with `main.js`.

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test -p rustpad-server --test run`
Expected: the new tests fail (`Run` is ignored or unknown).

- [ ] **Step 3: Port the run flow**

- [ ] **Step 4: Run all server tests**

Run: `cargo test -p rustpad-server`
Expected: all pass.

- [ ] **Step 5: Commit**

```bash
git add rustpad-server/src/rustpad.rs rustpad-server/tests/run.rs
git commit -m "Run code through go-judge and share results"
```

---

### Task 7: Frontend

**Files:**
- Copy from `feature/execution-runtime`: `src/rustpad.ts`, `src/App.tsx`, `src/Sidebar.tsx`, `src/RunOutput.tsx`
- Modify: `src/rustpad.ts`, `src/RunOutput.tsx`, `compose/compose.yml`

**Interfaces:**
- Consumes: the wire messages from Tasks 5 and 6.

- [ ] **Step 1: Copy the files**

Run: `git checkout feature/execution-runtime -- src/rustpad.ts src/App.tsx src/Sidebar.tsx src/RunOutput.tsx`. First check with `git diff main feature/execution-runtime -- src/App.tsx src/Sidebar.tsx` that those diffs only add the run features, since `main` may have moved on.

- [ ] **Step 2: Add `status`**

- `src/rustpad.ts`: `RunStage` gains `readonly status: string | null;`, and the `Runtime` doc comment becomes "A language this server can run, with the tool that runs it."
- `src/RunOutput.tsx` `summary()`: `const outcome = stage?.status ?? stage?.signal ?? \`exit ${stage?.code ?? "?"}\`;`
- `grep -rni piston src` returns nothing.

- [ ] **Step 3: Add `rustpad` to compose**

Add the `rustpad` service: `build: ..`, `image: localhost/rustpad:dev`, `environment: { GO_JUDGE_URL: http://runner:5050, RUST_LOG: info }`, `ports: ["127.0.0.1:3030:3030"]`, `depends_on: [runner]`. Update the header comment with the three commands from the spec's "Local stack" section.

- [ ] **Step 4: Check**

Run: `npm run check`
Expected: exits 0.

- [ ] **Step 5: Commit**

```bash
git add src/rustpad.ts src/App.tsx src/Sidebar.tsx src/RunOutput.tsx compose/compose.yml
git commit -m "Add Run button and output panel"
```

---

### Task 8: Smoke test, calibration and docs

**Files:**
- Create: `rustpad-server/tests/smoke.rs`, `compose/smoke-test.sh` (executable)
- Modify: `rustpad-server/src/runner/languages.rs` (calibrated limits), `README.md`

**Interfaces:**
- Consumes: `GoJudgeClient::{new, execute, probe}`.

`compose/smoke-test.sh` runs from any directory:
`cd "$(dirname "$0")/.." && GO_JUDGE_URL="${GO_JUDGE_URL:-http://127.0.0.1:5050}" RUST_LOG="${RUST_LOG:-rustpad_server=debug}" cargo test -p rustpad-server --release --test smoke -- --ignored --nocapture --test-threads=1`

Every test in `tests/smoke.rs` is `#[ignore]` and reads `GO_JUDGE_URL`.

- [ ] **Step 1: Write the smoke tests**

- `smoke_probes`: `probe` succeeds for all six ids; print each version.
- `smoke_hello_<id>` for each of the six: `execute` a hello world printing `hello from <id>`; assert `run.stdout == "hello from <id>\n"` and `run.code == Some(0)`; print the wall time.
- `smoke_java_solution`: `package demo; public class Solution` that sorts a list and prints it → expected stdout.
- `smoke_kotlin_program`: a data class, a `map`/`filter` chain and string templates → expected stdout.
- `smoke_limits`: Python `while True: pass` → `status == Some("Time limit exceeded")`; Python `bytearray(1 << 30)` → `status == Some("Memory limit exceeded")`; Python socket to `1.1.1.1:80` → non-zero `code`; Python writing 200 MB to `/w/big` → non-zero `code`, followed by a working `smoke_hello_python`-style call; Python `print("x" * 10_000_000)` → `truncated`.

- [ ] **Step 2: Start the stack and run the smoke tests**

Run: `./build.sh && podman compose -f compose/compose.yml up --build -d && compose/smoke-test.sh`
Expected: all smoke tests pass. If one fails, fix `runner/Dockerfile`, `runner/mount.yaml` or the table (env, flags), rebuild, and rerun.

- [ ] **Step 3: Calibrate the limits**

From the `debug!` lines, read each stage's CPU `time`, `runTime` and peak `memory`, over at least three runs after a fresh `podman compose restart runner`. Set each limit to at least 2× the worst observed value, keeping the spec's run defaults (3 s CPU, 256 MiB) unless a hello world measures above half of them. Kotlin's compile clock limit must cover the cold first compile; raise the 30 s request timeout only if compile clock plus run clock plus 5 s exceeds it. Rerun `compose/smoke-test.sh` and `cargo test -p rustpad-server`.

- [ ] **Step 4: Write the README section**

Add a `GO_JUDGE_URL` and `GO_JUDGE_TOKEN` bullet under "Configuration". Add a "Code execution" section with:
- A rootful Podman machine as the requirement on a Mac.
- The three commands to build, start and smoke-test the stack.
- `http://localhost:3030`, the Run button and `Ctrl/Cmd+Enter`.
- A table of the six languages with version, measured compile time and run time.
- The runner image size from Task 2 (or the rebuilt size).
- "Runner in production", with the five requirements from the spec's Security section.

- [ ] **Step 5: Commit**

```bash
git add rustpad-server/tests/smoke.rs compose/smoke-test.sh rustpad-server/src/runner/languages.rs README.md
git commit -m "Add runner smoke test, calibrate limits and document code execution"
```

---

### Task 9: Manual checklist

**Files:** none, unless a check fails.

- [ ] **Step 1: Run the checklist on the compose stack at `http://localhost:3030`**

- Hello world in each of the six languages from the Run button, and once with `Ctrl/Cmd+Enter`.
- A C++ compile error shows the compiler output and "compile failed".
- `while True: pass` shows "Time limit exceeded" at about 3 s.
- A fork bomb is contained, and the next run works.
- Printing 10 MB shows `[output truncated]`.
- Printing `<img src=x onerror=alert(1)>` and ANSI escapes shows literal text.
- Two browsers see the same result; a late joiner sees the last output.
- `podman compose -f compose/compose.yml stop runner` gives "Code runner is unavailable" on Run; after `start runner`, runs work again.
- Markdown shows a disabled Run button with "No runtime for markdown".

- [ ] **Step 2: Clean-up check**

Run: `git grep -ni piston -- ':!docs/superpowers'`
Expected: no output.
