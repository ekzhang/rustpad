FROM ekidd/rust-musl-builder:1.51.0 as backend
WORKDIR /home/rust/src
COPY . .
RUN cargo test --release
RUN cargo build --release

FROM node:alpine as frontend
WORKDIR /usr/src/app
COPY package.json package-lock.json ./
RUN npm ci
COPY . .
RUN npm run build

FROM scratch
COPY --from=frontend /usr/src/app/dist dist
COPY --from=backend /home/rust/src/target/x86_64-unknown-linux-musl/release/rustpad-server .
USER 1000:1000
CMD [ "./rustpad-server" ]
