# syntax=docker/dockerfile:1

# Build from repository root:
#   docker build -f deploy/Dockerfile --build-arg EXAMPLE_PKG=hello --build-arg EXAMPLE_BIN=hello .
#
# Runtime listens on 0.0.0.0:3000 (app.listen).
# Demo `hello` calls configure() and loads /app/sova.toml when present.

ARG RUST_IMAGE=rust:1-bookworm
ARG RUNTIME_IMAGE=debian:bookworm-slim

FROM ${RUST_IMAGE} AS builder
WORKDIR /src

ARG EXAMPLE_PKG=hello
ARG EXAMPLE_BIN=hello

# Cache dependency crates when manifests change less often than sources.
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
COPY plugins ./plugins
COPY examples ./examples

RUN cargo build --release -p "${EXAMPLE_PKG}" \
    && cp "target/release/${EXAMPLE_BIN}" /src/app

FROM ${RUNTIME_IMAGE} AS runtime
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /src/app /app/app
COPY deploy/sova.production.toml /app/sova.toml

ENV SOVA_ENV=production \
    RUST_LOG=info

EXPOSE 3000
USER nobody
CMD ["/app/app"]
