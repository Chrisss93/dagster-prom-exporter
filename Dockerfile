FROM registry.gitlab.com/chrisss93/rust-ci/chef:1.74.0 AS builder
# FROM rust:1.71.0 as builder
ENV RUSTFLAGS='-C link-arg=-fuse-ld=lld -C target-feature=-crt-static'
LABEL stage=builder
WORKDIR /build

RUN apt-get update && \
  apt-get install -y --no-install-recommends pkg-config libssl-dev && \
  rm -rf /var/lib/apt/lists/*

COPY Cargo.toml Cargo.lock ./
RUN cargo chef prepare --bin app
RUN cargo chef cook --release --bin app

# cargo chef modifies the toml+lock files so restore the originals first
COPY Cargo.toml Cargo.lock ./
COPY src src
COPY graphql graphql
RUN cargo build --locked --release --bin app

FROM gcr.io/distroless/cc-debian11:nonroot
COPY --from=builder /build/target/release/app /app
ENTRYPOINT ["/app"]
