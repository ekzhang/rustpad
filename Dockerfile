FROM rust:alpine AS backend
WORKDIR /home/rust/src
RUN apk --no-cache add musl-dev openssl-dev
COPY . .
RUN cargo test --release
RUN cargo build --release

FROM rust:slim-bookworm AS wasm
WORKDIR /home/rust/src
RUN apt-get update && apt-get install -y curl
ADD https://wasm-bindgen.github.io/wasm-pack/installer/init.sh /tmp/wasm-pack.install.sh
RUN sh /tmp/wasm-pack.install.sh && rm /tmp/wasm-pack.install.sh
COPY . .
RUN wasm-pack build rustpad-wasm

FROM node:lts-alpine AS frontend
WORKDIR /usr/src/app
COPY package.json package-lock.json ./
COPY --from=wasm /home/rust/src/rustpad-wasm/pkg rustpad-wasm/pkg
RUN npm ci
COPY . .
ARG GITHUB_SHA
ENV VITE_SHA=${GITHUB_SHA}
RUN npm run check
RUN npm run build

FROM scratch
COPY --from=frontend /usr/src/app/dist dist
COPY --from=backend /home/rust/src/target/release/rustpad-server .
USER 1000:1000
CMD [ "./rustpad-server" ]
