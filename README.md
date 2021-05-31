# Rustpad

**Rustpad** is an _efficient_ and _minimal_ open-source collaborative text
editor using the operational transformation (OT) algorithm. It has a client-side
web interface that communicates by WebSocket with a central server storing
in-memory data structures.

The backend is written in Rust, using
[Warp](https://github.com/seanmonstar/warp) and the open source
[operational-transform](https://github.com/spebern/operational-transform-rs)
library, which is a port of
[ot.js](https://github.com/Operational-Transformation/ot.js). The frontend is
written in React and interfaces with
[Monaco](https://github.com/microsoft/monaco-editor), the same text editor that
powers VS Code.
