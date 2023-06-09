# syntax=docker/dockerfile:experimental

FROM rust:1.47 AS builder
WORKDIR /usr/src/app

RUN apt-get update && apt-get install -y cmake gcc librdkafka-dev libsasl2-dev libpq-dev

COPY Cargo.toml Cargo.lock ./
CMD cargo update --frozen --locked

run echo 'fn main() {}' >> dummy.rs
RUN echo '[[bin]]\nname = "app"\npath = "dummy.rs"' >> Cargo.toml
RUN cargo build --release
run rm dummy.rs

COPY . ./

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    cargo build --release

FROM debian:buster
RUN apt-get update && apt-get install -y ca-certificates libsasl2-2 libpq5 openssl
# RUN apk add cmake librdkafka libsasl libpq
# RUN apk --no-cache add ca-certificates
# RUN addgroup -S app && adduser -S -G app app

# Copy and mount postgres ssl certificates
VOLUME /root/.postgresql

COPY --from=builder \
    /usr/src/app/target/release/server \
    /usr/src/app/target/release/importer \
    /usr/src/app/target/release/importer_redis \
    /usr/src/app/target/release/invalidator_redis \
    /usr/src/app/target/release/migrate \
    /usr/local/bin/

CMD /usr/local/bin/server
