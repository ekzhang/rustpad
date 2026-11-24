# Code execution with go-judge – design

Date: 2026-11-19
Status: approved

## Why

The `feature/execution-runtime` branch runs pad code through Piston. It works,
but the image with runtimes baked in was the problem: some Piston runtime
packages would not install on arm64, the build took a long time, and the image
came out very large.

[go-judge](https://github.com/criyle/go-judge) is lower level. It runs commands
in a sandbox and knows nothing about languages, so the runner image is just
go-judge plus compilers from normal Debian packages. Those exist for arm64 and
amd64 alike. The goal is a runner image that builds on both architectures,
builds fast and reliably, and is lighter than the Piston one.

The user-facing feature does not change. The Piston design
(`docs/superpowers/specs/2026-10-09-code-execution-design.md` on
`feature/execution-runtime`) still holds for the run model, wire protocol,
UI, threat model and tests, except where this document says otherwise.

## Goals

- Run Python, C, C++, Java, Kotlin and JavaScript from a pad, with the same
  shared run model and UI as the Piston branch.
- One runner image, built from this repo with `podman compose`, for the
  host's architecture: `linux/arm64` or `linux/amd64`.
- The local compose stack builds and starts Rustpad and the runner together,
  on a Mac with Apple silicon, without Rosetta.

## Non-goals

- Any change to what users see, apart from the limit status line below.
- More languages than the six above. Adding one later should mean a table
  entry plus a package in the image.
- Rate limiting, stdin, arguments, multiple files, saved results, phones (as
  before).
- Kubernetes manifests (infra repo, as before).
- CI for the runner image: building, testing and publishing it. The user
  deferred this; the local compose build is the only build path for now.

## Anti-goals

- A runner image that only builds or only works on one architecture.
- A language that shows a Run button but whose compiler is missing from the
  image.
- Weakening the sandbox compared with Piston: no network, CPU, memory,
  process and output limits on every run.

## Spike results (2026-11-19)

A throwaway image (`criyle/go-judge` v1.14.1 plus `python3` and `g++`) ran on
the user's Mac: Podman machine `podman-machine-default`, rootful, Fedora
kernel 7.2, arm64, container started with `--privileged --shm-size=256m`.

| Check | Result |
|---|---|
| Python hello world | Accepted, 8 ms |
| C++ compile with `copyOutCached`, then run by `fileId` | compile 171 ms, run 1 ms |
| C++ compile error | `Nonzero Exit Status`, exit 1, message in stderr |
| `while True: pass`, cpuLimit 3 s | `Time Limit Exceeded` at 3.1 s, exit 9 |
| `sleep(30)`, clockLimit 6 s | `Time Limit Exceeded` at 6 s |
| Socket to 1.1.1.1 | fails; the sandbox has no network |
| 1 GiB allocation, memoryLimit 256 MiB | `Memory Limit Exceeded` |
| Fork bomb, procLimit 50 | `BlockingIOError`, contained |
| 10 MB stdout, collector `max` 64 KiB | `Output Limit Exceeded`, output cut |

The stock image is 168 MB (Debian 13). With `python3` and `g++` it was 490 MB.
A rootless machine was not tried.

## Decisions

| Topic | Decision |
|---|---|
| Runner | go-judge, self-hosted, REST API |
| Integration | Rustpad server calls go-judge; the browser never reaches it (as before) |
| Client | In-house, on `reqwest` with rustls (as before) |
| Languages | Fixed table in Rustpad; versions probed from the image |
| Runner image | Built in this repo from `runner/Dockerfile`, locally through `podman compose`; no CI job |
| Branch | `feature/execution-go-judge` off `main`; reusable commits come over from `feature/execution-runtime` |

## What comes over from `feature/execution-runtime`

Reuse these as they are, or with Piston names removed:

- Frontend: `src/rustpad.ts` types and callbacks, `src/App.tsx` run state and
  `Ctrl/Cmd+Enter`, `src/Sidebar.tsx` Run button, `src/RunOutput.tsx`.
- Server run flow in `rustpad-server/src/rustpad.rs`: `ClientMsg::Run`, the
  four `ServerMsg` variants, `RunState`, `start_run`, replay in
  `send_initial`, the `watch` channel for the runtime list, the 64 KiB code
  limit.
- From `piston.rs`: `Runtime`, `RunStage`, `ExecOutput`, `RunError` and its
  fixed messages, `truncate_utf8`, the `Runner` refresher with retry backoff,
  proxy bypass for runner requests.
- Build: host-built server binary (`build.sh`, Dockerfile that only packages
  it), Podman compose stack. The branch's CI change that runs `build.sh`
  before packaging comes over too, so the existing Rustpad image job keeps
  working with the new Dockerfile. No runner job gets added.
- Tests: the fake-server pattern in `tests/common/piston.rs` and the cases in
  `tests/run.rs`.

Drop: the Piston client, runtime resolution against Piston's list, the
override table, semver comparison, `PISTON_URL`, the Piston docs and README
sections.

## Server

### `rustpad-server/src/runner.rs` (replaces `piston.rs`)

`GoJudgeClient` holds a `reqwest::Client`, a base URL and an optional token. It
calls only:

- `POST /run` to run one command.
- `DELETE /file/{id}` to drop a cached binary after the run stage.

Connect timeout 2 s. Total request timeout 30 s, enough for the longest
compile clock limit plus queueing. The response body cap (4 MiB) and the
fixed user-facing error messages stay as on the Piston branch.

### Language table

A `const` table in `runner.rs`, one entry per Monaco language ID:

| Monaco ID | Shown as | Source file | Compile | Run | Version probe |
|---|---|---|---|---|---|
| `python` | `python` | `main.py` | – | `python3 main.py` | `python3 --version` |
| `c` | `gcc` | `main.c` | `gcc -O2 -std=c17 main.c -o main -lm` | `./main` | `gcc -dumpfullversion` |
| `cpp` | `g++` | `main.cpp` | `g++ -O2 -std=c++20 main.cpp -o main` | `./main` | `g++ -dumpfullversion` |
| `java` | `java` | see below | `javac <File>.java` | `java -cp . <Class>` | `java -version` |
| `kotlin` | `kotlin` | `main.kt` | `kotlinc main.kt -include-runtime -d main.jar` | `java -jar main.jar` | `kotlinc -version` |
| `javascript` | `node` | `main.js` | – | `node main.js` | `node --version` |

Commands use absolute paths in the real table. Exact flags are the builder's
to settle. Each entry also holds its own limits (below) and any JVM flags.

Java: `javac` insists that a public class lives in a file of the same name.
Rustpad names the file after the first `public class` it finds in the source,
falling back to `Main`, and runs that class. `copyOutCached` needs exact file
names, and `javac` may emit several `.class` files, so the compile stage
packs them into one jar that the run stage uses (builder's call how).

### Limits

go-judge `cpuLimit` counts CPU time across all threads, and JVMs use many.
Limits are therefore per language and per stage. Starting values:

| Stage | CPU | Wall clock | Memory | Processes |
|---|---|---|---|---|
| Run, native and Python/Node | 3 s | 5 s | 256 MiB | 64 |
| Run, JVM | 5 s | 5 s | 256 MiB | 128 |
| Compile, gcc/g++ | 10 s | 15 s | 512 MiB | 64 |
| Compile, javac/kotlinc | 20 s | 15 s | 512 MiB | 128 |

JVM commands get an explicit `-Xmx` below the memory limit, since the JVM
inside the sandbox may size its heap from the whole host. Each stage collects
stdout and stderr with `max` 64 KiB.

These are first guesses. The builder calibrates them against hello world and a
small real program in each language on the real image, and records the
measured times in the README. Kotlin's compiler is the slow one; the user
accepted that a Kotlin run takes several seconds.

### Running

`execute(language, source) -> Result<ExecOutput, RunError>`:

1. Interpreted: one `/run` with the source in `copyIn`.
2. Compiled: `/run` the compile command with `copyOutCached` for the output
   files. On a non-zero exit, return `compile` only. Otherwise `/run` the run
   command with the cached files in `copyIn` by `fileId`, then `DELETE` each
   file, even when the run fails.

Mapping a go-judge result to `RunStage`:

- `stdout`, `stderr`: from `files`, cut on a UTF-8 boundary;
  `truncated: true` when go-judge reports `Output Limit Exceeded` or the text
  was cut.
- `code`: `exitStatus` for `Accepted` and `Nonzero Exit Status`.
- `signal`: the signal name for `Signalled`, and `SIGKILL` for time and
  memory kills.
- `status` (new, optional): "Time limit exceeded", "Memory limit exceeded",
  or "Blocked system call" for `Dangerous Syscall`. `null` otherwise.
- `Internal Error` or `File Error` becomes `RunError`, logged in full, shown as
  "Code runner returned an invalid response".

### Runtime list

The `Runner` refresher stays, but instead of fetching Piston's list it runs
each table entry's version probe through go-judge. A language goes into the
list only when its probe succeeds; the version is the first `\d+(\.\d+)+`
in the probe's output. So a compiler missing from the image hides that
language instead of showing a broken Run button. When every probe fails
(go-judge down), the refresher keeps the previous list and retries with the
existing backoff.

A language that passed before stays in the list until its probe fails on
three refreshes in a row, so one slow `kotlinc -version` under load does not
make the Run button vanish (my call).

`Runtime` on the wire stays `{ language, runtime, version }`, with `runtime`
set to the "Shown as" name.

### Configuration

- `GO_JUDGE_URL`, for example `http://runner:5050`. Unset means code
  execution is off, as `PISTON_URL` did.
- `GO_JUDGE_TOKEN`, optional. Sent as a bearer token to match go-judge's
  `-auth-token`.

## Wire protocol

Unchanged from the Piston design, except `RunStage` gains
`status: string | null`.

## Frontend

As on the Piston branch, plus: when a stage has `status`, the output header
shows it in place of the exit code or signal, for example
"Run by Ana · python 3.13.5 · 3104 ms · Time limit exceeded". Comments and
names that say Piston change to neutral wording.

## Runner image

`runner/Dockerfile`:

- `FROM` the `criyle/go-judge` image pinned by its multi-arch digest. It is
  Debian 13 and already holds go-judge and a `mount.yaml`.
- `apt-get install --no-install-recommends`: `python3`, `gcc`, `g++`,
  `openjdk-21-jdk-headless`, `nodejs`.
- Kotlin compiler: the release zip from GitHub, pinned by version and
  SHA-256. It is plain JVM code, the same on both architectures.
- Our own `mount.yaml`, trimmed to what these languages need (the stock one
  mounts paths for Pascal, Mono and GHC that we don't ship).
- go-judge flags: `-http-addr :5050`, a `-file-timeout` so cached binaries
  expire even if Rustpad never deletes them, and `-auth-token` from the
  environment when set. gRPC off.
- Package versions are whatever Debian 13 holds on the day of the build. The
  digest pin fixes go-judge and the base layer only; compiler versions move
  with Debian point releases and security updates. The smoke test (below)
  guards against a broken update. Pinning to `snapshot.debian.org` would fix
  versions but makes builds slower and more fragile, which works against the
  goal (my call).

The README records the image size for each architecture. No hard size target;
the comparison is with the Piston image.

## Local stack

`compose/compose.yml` gets back a second service:

- `runner`: built from `runner/`, `privileged: true` with a comment on why,
  `shm_size: 256m`, image tag `localhost/rustpad-runner:dev`, port published
  on `127.0.0.1:5050` only (for the smoke test).
- `rustpad`: `GO_JUDGE_URL: http://runner:5050`, `depends_on: runner`.

The README documents the Mac setup the spike confirmed: rootful Podman
machine. Rosetta is not needed.

The local workflow, from the repository root:

```
./build.sh
podman compose -f compose/compose.yml up --build -d
compose/smoke-test.sh
```

`compose/smoke-test.sh` (new, my call) runs one hello world per language
straight against the runner's API and fails on the first language that does
not print the expected line. It stands in for a CI check until CI exists.

The Dockerfile must stay free of architecture-specific steps, so the same
file builds on amd64. Only arm64 gets run and checked during this work;
amd64 is checked when CI arrives.

## Security

The Piston threat model and layers still apply, with go-judge in place of
`isolate`. Production requirements for the infra repo change only in detail:

1. Dedicated node pool, no cloud permissions on the nodes (as before).
2. Own namespace at PodSecurity level `privileged`; `privileged: true`,
   no service account token, no hostPath, pod CPU and memory limits.
3. NetworkPolicy: ingress on port 5050 only from Rustpad pods; all egress
   denied, DNS included.
4. `GO_JUDGE_TOKEN` set on both sides.
5. ClusterIP Service only.

go-judge exposes `POST /file` and `GET /file`; Rustpad never calls them, and
only Rustpad pods can reach the runner.

## Testing

Unit tests in `runner.rs`:

- Result mapping for each go-judge status, including `status` and `signal`.
- Version parsing from each probe's real output format.
- Java file and class naming: public class, no public class, several classes.
- Truncation and error mapping (reused).

Integration tests (`tests/run.rs`) use a fake go-judge on `127.0.0.1:0`, as
the fake Piston did, and cover the same cases. In addition:

- A compiled run makes compile, run and delete calls in order, and the run
  call references the compile call's `fileId`.
- A compile failure makes no run call.
- A failed probe leaves that language out of `Runtimes`.

Manual checklist on the compose stack: the Piston checklist, for all six
languages, on arm64 at least, plus a Kotlin and a Java program that print
more than hello world.

## Left to the builder

- Exact compiler flags, JVM flags, and the calibrated limits.
- The trimmed `mount.yaml` contents and environment for each language
  (`PATH`, `HOME`, and what the JVM needs).
- How to bring the old commits over (cherry-pick or rewrite), as long as no
  Piston code is left.
- Module and type names beyond those given here.
