FROM rust:1.73.0 AS builder
RUN cargo new --bin app
WORKDIR /app
COPY Cargo.* ./
RUN cargo build --release
COPY src/*.rs ./src/.
RUN touch -a -m ./src/main.rs
RUN cargo build --release

FROM debian:stable-slim
RUN apt update && apt install -y openssl ca-certificates
WORKDIR /app
COPY --from=builder /app/target/release/war_score /app/war_score
CMD "/app/war_score"
