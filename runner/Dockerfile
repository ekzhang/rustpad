FROM debian:trixie-slim AS kotlin
ARG KOTLIN_VERSION=2.4.21
ARG KOTLIN_SHA256=7cc140e76daf416a0424a5557f99afd7ca89ebd463ea1101261e3e01c33e75cd
RUN apt-get update \
    && apt-get install -y --no-install-recommends curl ca-certificates unzip \
    && rm -rf /var/lib/apt/lists/*
RUN curl -fsSL -o /kotlin.zip \
        "https://github.com/JetBrains/kotlin/releases/download/v${KOTLIN_VERSION}/kotlin-compiler-${KOTLIN_VERSION}.zip" \
    && echo "${KOTLIN_SHA256}  /kotlin.zip" | sha256sum -c - \
    && unzip -q /kotlin.zip -d / \
    && rm /kotlin.zip

# go-judge v1.14.1, multi-arch index digest.
FROM docker.io/criyle/go-judge:v1.14.1@sha256:da459bed95efca8f322d98598fed5822379b12bf2a0f83c17b34ad55bca19b30
RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        python3 gcc g++ libc6-dev openjdk-21-jdk-headless nodejs \
    && rm -rf /var/lib/apt/lists/*
# The sandbox sees only /usr, /bin and /lib*, so Kotlin lives under /usr/local.
COPY --from=kotlin /kotlinc /usr/local/lib/kotlinc
RUN ln -s /usr/local/lib/kotlinc/bin/kotlinc /usr/local/bin/kotlinc
COPY mount.yaml /opt/mount.yaml

# The base image sets ENTRYPOINT ["/opt/go-judge"] and WORKDIR /opt.
# Set ES_AUTH_TOKEN to require a bearer token (go-judge flag -auth-token).
CMD ["-http-addr", ":5050", "-file-timeout", "10m"]
