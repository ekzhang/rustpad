# Rustpad

**Rustpad** is an _efficient_ and _minimal_ open-source collaborative text
editor based on the operational transformation (OT) algorithm. Rustpad provides
an online real-time code editor that communicates via WebSocket with a central
server storing in-memory data structures.

The backend is written in Rust, using
[Warp](https://github.com/seanmonstar/warp) and the open source
[operational-transform](https://github.com/spebern/operational-transform-rs)
library, which is a port of
[ot.js](https://github.com/Operational-Transformation/ot.js). The frontend is
written in React and interfaces with
[Monaco](https://github.com/microsoft/monaco-editor), the same text editor that
powers VS Code.

## Development setup

To run this application, you need to install Rust, `wasm-pack`, and Node.js.
Then, build the WebAssembly portion of the app:

```
wasm-pack build rustpad-core
```

When that is complete, you can install dependencies for the frontend React
application with:

```
npm install
```

Finally, compile and run the backend web server with:

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

To run unit tests and integration tests for the server, use the standard
`cargo test` command. For the WebAssembly component, you can run tests in a
headless browser with

```
wasm-pack test rustpad-core --chrome --headless
```

## Deployment

Rustpad is distributed as a single ~10 MB Docker image, which is built from the
`Dockerfile` in this repository. We use GitHub Actions to automatically build
new images on every push to the `main` branch.

We continuously deploy a single container of this image to the
[DigitalOcean App Platform](https://www.digitalocean.com/products/app-platform/).
