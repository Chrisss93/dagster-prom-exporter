FROM registry.gitlab.com/chrisss93/rust-ci/chef:1.69.0 AS builder
LABEL stage=builder
WORKDIR /app
RUN apt-get update && \
  apt-get install -y --no-install-recommends pkg-config libssl-dev && \
  rm -rf /var/lib/apt/lists/*
COPY Cargo.toml Cargo.lock ./
RUN cargo chef prepare && cargo chef cook --release
COPY . .
RUN cargo build --locked --release --bin app

FROM gcr.io/distroless/cc-debian11:nonroot
COPY --from=builder /app/target/release/app /app
ENTRYPOINT ["/app"]
