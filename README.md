![Apex-RS Logo](./static/logo_title.svg)

# Apex-RS: All Programs EXecute Rdf Source

- Performant RDF Triple store / Graph Database written in Rust.
- Uses [linked-delta](https://github.com/ontola/linked-delta) events for communicating state changes.
- Currently supports reading these linked-delta events from both kafka and redis, but the architecture allows for alternatives.
- Provides a [Triple Pattern Fragments](https://linkeddatafragments.org/specification/triple-pattern-fragments/) and a [Bulk-API](https://github.com/ontola/bulk-api) endpoint for RDF queries.
- Serializes to various RDF formats (Turtle, N-Triples, [HexTuples](https://github.com/ontola/hextuples)).

## Running with docker-compose

1. Install [docker-compose](https://docs.docker.com/compose/install/)
1. Make sure to [enable Buildkit](https://www.docker.com/blog/faster-builds-in-compose-thanks-to-buildkit-support/). (put `export COMPOSE_DOCKER_CLI_BUILD=1` in your `.profile` file)
1. `docker-compose up` will spin up kafka, postgres and apex-rs. It will create `./persist` directory for kafka and postgres.
1. `docker-compose  `
1. Visit

## Running locally

1. Set up [postgres](https://www.postgresql.org/docs/current/tutorial-install.html) and kafka.
1. Copy the template env file `cp template.env .env`.
1. Fill in the `DATABASE_URL` with your PostGres URL (e.g. `postgres://localhost`)
1. When using SSL with postgres
    1. Download and rename database SSL certificates to `certs` folder, see Dockerfile.
    1. Change the cert permissions `chmod -R 700 certs`
1. Run `cargo install diesel_cli --no-default-features --features "postgres"` and run `diesel setup --database-url=postgres://localhost` to initialize the db schema.
1. Add a record to _apex_config with key "seed" and a random 32 bit integer string value. You might want to use a tool such as PGAdmin.

Building the project manually
- `cargo build`
Building the project via docker
- `DOCKER_BUILDKIT=1 docker build . -t apex-rs:latest`

Running the project manually
- `cargo run --bin server`
- `cargo run --bin importer`

Auto rebuild on file changes
- `cargo install cargo-watch`
- `cargo watch run --bin server`

Running the project via docker (make sure to [enable Buildkit](https://www.docker.com/blog/faster-builds-in-compose-thanks-to-buildkit-support/))
- `docker run -t apex-rs:latest /usr/local/bin/server` (default without arg)
- `docker run -t apex-rs:latest /usr/local/bin/importer`

## Custom pages

You can customize the static welcome page + assets by creating a `./static_custom` folder.

```sh
cp -R static static_custom
```

## Troubleshooting

### Compiling in MacOS:

```
brew install autoconf automake libtool openssl

git clone https://github.com/cyrusimap/cyrus-sasl.git
cd cyrus-sasl/
sh ./autogen.sh
make
sudo make install
cd ..
OPENSSL_ROOT_DIR=/usr/local/opt/openssl cargo run --bin server
```
