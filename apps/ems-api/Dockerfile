FROM rust:1.92-bookworm AS builder

WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY apps/ems-api/Cargo.toml apps/ems-api/Cargo.toml
COPY crates crates
COPY apps apps

RUN cargo build --release -p ems-api

FROM debian:bookworm-slim

RUN apt-get update \
  && apt-get install -y --no-install-recommends ca-certificates \
  && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/ems-api /usr/local/bin/ems-api

EXPOSE 8080

ENV RUST_LOG=info

CMD ["/usr/local/bin/ems-api"]

