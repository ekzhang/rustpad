# Rustpad

**Rustpad** is an _efficient_ and _minimal_ open-source collaborative text
editor based on the operational transformation algorithm. It lets users
collaborate in real time while writing code in their browser. Rustpad is
completely self-hosted and fits in a tiny Docker image, no database required.

<p align="center">
<a href="https://rustpad.io/">
<img src="https://i.imgur.com/WjU5UrP.png" width="800"><br>
<strong>rustpad.io</strong>
</a>
</p>

The server is written in Rust using the
[warp](https://github.com/seanmonstar/warp) web server framework and the
[operational-transform](https://github.com/spebern/operational-transform-rs)
library. We use [wasm-bindgen](https://github.com/rustwasm/wasm-bindgen) to
compile text operation logic to WebAssembly code, which runs in the browser. The
frontend is written in TypeScript using [React](https://reactjs.org/) and
interfaces with [Monaco](https://github.com/microsoft/monaco-editor), the text
editor that powers VS Code.

Architecturally, client-side code communicates via WebSocket with a central
server that stores in-memory data structures. This makes the editor very fast,
allows us to avoid provisioning a database, and makes testing much easier. The
tradeoff is that documents are transient and lost between server restarts, or
after 24 hours of inactivity.

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

While the backend is running, open another shell and run the following command
to start the frontend portion.

```
npm start
```

This command will open a browser window to `http://localhost:3000`, with hot
reloading on changes.

## Testing

To run integration tests for the server, use the standard `cargo test` command.
For the WebAssembly component, you can run tests in a headless browser with

```
wasm-pack test rustpad-wasm --chrome --headless
```

## Deployment

Rustpad is distributed as a single 12 MB Docker image, which is built
automatically from the `Dockerfile` in this repository. You can pull the latest
version of this image from Docker Hub.

```
docker pull ekzhang/rustpad
```

(You can also manually build this image with `docker build -t rustpad .` in the
project root directory.) To run locally, execute the following command, then
open `http://localhost:3030` in your browser.

```
docker run --rm -dp 3030:3030 ekzhang/rustpad
```

We deploy a public instance of this image using
[DigitalOcean App Platform](https://www.digitalocean.com/products/app-platform/).

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
