# syntax=docker/dockerfile:1
#
# Template for YOUR Sova application (not the framework repo).
# Copy this file to your app root (next to Cargo.toml), then:
#   docker compose up --build
#
# Expects a normal Cargo binary crate:
#   Cargo.toml / Cargo.lock / src/ …
# Optional: views/, public/, migrations/ — uncomment COPY lines below.

ARG RUST_IMAGE=rust:1-bookworm
ARG RUNTIME_IMAGE=debian:bookworm-slim

FROM ${RUST_IMAGE} AS builder
WORKDIR /src

# Binary name from [[bin]] / package name (override: --build-arg APP_BIN=…).
ARG APP_BIN=myapp

COPY Cargo.toml Cargo.lock ./
COPY src ./src
# COPY views ./views
# COPY public ./public
# COPY migrations ./migrations
# COPY templates ./templates

RUN cargo build --release \
    && cp "target/release/${APP_BIN}" /src/app-bin

FROM ${RUNTIME_IMAGE} AS runtime
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --uid 10001 --home /app --shell /usr/sbin/nologin sova

WORKDIR /app
COPY --from=builder /src/app-bin /app/app
# Bake a default config; compose can still mount over it.
COPY sova.toml /app/sova.toml

ENV SOVA_ENV=production \
    RUST_LOG=info

EXPOSE 3000
USER sova
CMD ["/app/app"]
