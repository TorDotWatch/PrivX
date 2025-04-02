FROM rust:1.85-alpine
WORKDIR /app
COPY Cargo.toml ./
COPY resources ./resources
COPY src ./src
COPY templates ./templates

RUN apk add pkgconfig openssl-dev

# Install musl-tools to make many crates compile successfully
RUN apk add --no-cache musl-dev

# Uncomment for development
# RUN cargo install cargo-watch

RUN cargo build --release

EXPOSE 8181
# Uncomment for development live code reload
# CMD ["cargo", "watch", "-x", "run"]

CMD ["/app/target/release/IncognitoBin_server"]
