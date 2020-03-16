FROM rust:1-slim-buster

CMD cargo run

RUN apt-get update && apt-get install -y pkg-config cmake g++ libssl-dev librdkafka-dev libsasl2-dev libpq-dev

# Copy and mount postgres ssl certificates
VOLUME /root/.postgresql
COPY certs/postgresql.crt certs/postgresql.key certs/root.crt /root/.postgresql/

WORKDIR /usr/src/apex-rs

# Copy all files except certs
COPY migrations migrations
COPY src src
COPY Cargo.lock Cargo.toml diesel.toml .env ./

RUN cargo build
