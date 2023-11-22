FROM registry.gitlab.com/chrisss93/rust-ci/chef:1.74.0 AS builder
LABEL stage=builder
WORKDIR /app

RUN apt-get update && \
  apt-get install -y --no-install-recommends pkg-config libssl-dev && \
  rm -rf /var/lib/apt/lists/*

COPY Cargo.toml Cargo.lock ./
COPY .cargo/config.toml .cargo/config.toml
RUN cargo chef prepare --bin app
RUN cargo chef cook --release --bin app

# cargo chef modifies the toml+lock files so restore the originals first
COPY Cargo.toml Cargo.lock ./
COPY src src
COPY graphql graphql
RUN cargo build --locked --release --bin app

FROM gcr.io/distroless/cc-debian11:nonroot
COPY --from=builder /app/target/release/app /app
ENTRYPOINT ["/app"]
