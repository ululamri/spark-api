FROM rust:1-bookworm AS builder

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo build --release

FROM debian:bookworm-slim AS runtime

RUN apt-get update \
  && apt-get install -y --no-install-recommends ca-certificates curl \
  && rm -rf /var/lib/apt/lists/*

RUN useradd --create-home --shell /usr/sbin/nologin spark
WORKDIR /app

COPY --from=builder /app/target/release/spark-api /usr/local/bin/spark-api

ENV SPARK_API_HOST=0.0.0.0
ENV SPARK_API_PORT=8787

EXPOSE 8787

USER spark

HEALTHCHECK --interval=30s --timeout=5s --retries=3 \
  CMD curl -fsS http://127.0.0.1:8787/health/live || exit 1

CMD ["spark-api"]
