# Rustpad

This is a fork of: [https://github.com/ekzhang/rustpad](https://github.com/ekzhang/rustpad)

See upstream readme for app description and most details.


## Development setup

To run this application, you need to install Rust, `wasm-pack`, and Node.js.
Then, build the WebAssembly portion of the app:

```
wasm-pack build rustpad-wasm
```

When that is complete, you can install dependencies for the frontend React
application:

```
npm install
```

Next, compile and run the backend web server:

```
cargo run
```

To turn on the Run button, start the compose runner (see
[Code execution](#code-execution)) and point the server at it:

```
podman compose -f compose/compose.yml up --build -d runner
GO_JUDGE_URL=http://127.0.0.1:5050 cargo run
```

While the backend is running, open another shell and run the following command
to start the frontend portion.

```
npm run dev
```

This command will open a browser window to `http://localhost:5173`, with hot
reloading on changes.

## Testing

To run integration tests for the server, use the standard `cargo test` command.
For the WebAssembly component, you can run tests in a headless browser with

```
wasm-pack test --chrome --headless rustpad-wasm
```

## Configuration

Although the default behavior of Rustpad is to store documents solely in memory
and collect garbage after 24 hours of inactivity, this can be configured by
setting the appropriate variables. The application server looks for the
following environment variables on startup:

- `EXPIRY_DAYS`: An integer corresponding to the number of days that inactive
  documents are kept in memory before being garbage collected by the server
  (default 1 day).
- `SQLITE_URI`: A SQLite connection string used for persistence. If provided,
  Rustpad will snapshot document contents to a local file, which enables them to
  be retained between server restarts and after their in-memory data structures
  expire. (When deploying a Docker container, this should point to the path of a
  mounted volume.)
- `PORT`: Which local port to listen for HTTP connections on (defaults to 3030).
- `RUST_LOG`: Directives that control application logging, see the
  [env_logger](https://docs.rs/env_logger/#enabling-logging) docs for more
  information.
- `GO_JUDGE_URL`: Base URL of the go-judge code runner, such as
  `http://runner:5050`. Code execution is off when this is unset.
- `GO_JUDGE_TOKEN`: Bearer token that Rustpad sends to the code runner
  (optional).

## Code execution

Rustpad runs the code in a pad through a [go-judge](https://github.com/criyle/go-judge)
runner. The runner image lives in `runner/`, and `compose/compose.yml` starts
it next to Rustpad.

On a Mac, the stack needs a rootful Podman machine:

```
podman machine stop
podman machine set --rootful
podman machine start
```

To build, start and smoke-test the stack, run from the repository root:

```
./build.sh
podman compose -f compose/compose.yml up --build -d
compose/smoke-test.sh
```

Open `http://localhost:3030`, then press the Run button or `Ctrl/Cmd+Enter`.

The runner supports these languages. Times are wall-clock, for a hello
world on an Apple Silicon Mac.

| Language   | Runtime | Version   | Compile | Run    |
| ---------- | ------- | --------- | ------- | ------ |
| Python     | python  | 3.13.5    | -       | 6 ms   |
| C          | gcc     | 14.2.0    | 24 ms   | < 1 ms |
| C++        | g++     | 14.2.0    | 0.21 s  | 1 ms   |
| Java       | java    | 21.0.13.1 | 0.26 s  | 18 ms  |
| Kotlin     | kotlin  | 2.4.21    | 2.6 s   | 22 ms  |
| JavaScript | node    | 20.19.2   | -       | 58 ms  |

The runner image is 979 MB on arm64.

### Runner in production

- Run the runner on a dedicated node pool whose nodes have no cloud
  permissions.
- Give the runner its own namespace at PodSecurity level `privileged`. Run the
  pod with `privileged: true`, no service account token, no hostPath volumes,
  and CPU and memory limits.
- Add a NetworkPolicy that allows ingress on port 5050 only from Rustpad pods
  and denies all egress, DNS included.
- Set `GO_JUDGE_TOKEN` on Rustpad and the same value in `ES_AUTH_TOKEN` on the
  runner.
- Expose the runner through a ClusterIP Service only.
- Set `ES_PARALLELISM` on the runner to the number of runs it may execute at
  once.
- Set the runner pod memory limit to at least `ES_PARALLELISM` × 1.3 GiB.

## Deployment

`build.sh` builds the frontend and a static Linux server binary on the host,
and the `Dockerfile` copies them into the image. The build runs on macOS or
Linux and needs `rustup`, `zig`, `cargo-zigbuild`, `wasm-pack` and `npm`.

```
./build.sh
podman build -t rustpad .
```

`build.sh` targets the host architecture. Set `ARCH` to pick others:

- `ARCH=amd64`
- `ARCH=arm64`
- `ARCH="amd64 arm64"`

To run the image, execute the following command, then open
`http://localhost:3030` in your browser.

```
podman run --rm -dp 3030:3030 rustpad
```

To turn on code execution, set `GO_JUDGE_URL` to a runner started as described
in [Code execution](#code-execution).

## In the media

- **July 11, 2021:** Featured in
  [Console 61 - The open-source newsletter](https://console.substack.com/p/console-61).
- **June 5, 2021:** Front-page
  [Hacker News post](https://news.ycombinator.com/item?id=27408326). Reddit
  discussions in [r/rust](https://www.reddit.com/r/rust/comments/nt4p9f/) and
  [r/programming](https://www.reddit.com/r/programming/comments/nt4ws7/).

<br>

<sup>
All code is licensed under the <a href="LICENSE">MIT license</a>.
</sup>
