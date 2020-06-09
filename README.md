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
1. For initial setup, start postgres `docker-compose run db`. This will create a `./persist` dir.
1. Setup the db with `docker-compose run diesel-cli`
1. Whilst in `diesel-cli`, run `diesel setup`
1. Exit this container `ctrl+c`
1. Add the seed row to postgres: `docker-compose exec db sh`, continue in "Add Seed"
1. `docker-compose up apex` will spin up redis, postgres and apex-rs. It will create `./persist` directory for redis and postgres.
1. Visit `localhost:8080`

## Add seed

Apex-RS requires a seed in its database.
You can set this using `psql` or a tool like PGAdmin.

1. `psql -U postgres`
1. `\c apex`
1. `INSERT INTO _apex_config (key, value) VALUES ('seed','14012979');`

## Running locally

1. Set up [postgres](https://www.postgresql.org/docs/current/tutorial-install.html) and [redis-server and redis-cli](https://redis.io/topics/quickstart).
1. Copy the template env file `cp template.env .env`.
1. Fill in the `DATABASE_URL` with your PostGres URL (e.g. `postgres://localhost`)
1. When using SSL with postgres
    1. Download and rename database SSL certificates to `certs` folder, see Dockerfile.
    1. Change the cert permissions `chmod -R 700 certs`
1. Run `cargo install diesel_cli --no-default-features --features "postgres"` and run `diesel setup --database-url=postgres://localhost` to initialize the db schema.
1. Add the seed (see "Add seed")

Building the project manually
- `cargo build`
Building the project via docker
- `DOCKER_BUILDKIT=1 docker build . -t apex-rs:latest`

Running the project manually
- `cargo run --bin server`
- `cargo run --bin importer_redis`

Running the project via docker (make sure to [enable Buildkit](https://www.docker.com/blog/faster-builds-in-compose-thanks-to-buildkit-support/))
- `docker run -t apex-rs:latest /usr/local/bin/server` (default without arg)
- `docker run -t apex-rs:latest /usr/local/bin/importer_redis`

## Loading deltas using Redis

Publish to the `cache` channel and use [HexTuples-ndjson](https://github.com/ontola/hextuples) for [linked-deltas](https://github.com/ontola/linked-delta).

```shis-12a

redis-cli
PUBLISH cache "[\"http://localhost:8080/test\", \"http://schema.org/birthDate\", \"1955-06-08\", \"http://www.w3.org/2001/XMLSchema#date\", \"\", \"http://purl.org/linked-delta/replace\"]"
```

## Custom pages

You can customize the static welcome page + assets by creating a `./static_custom` folder.

```sh
cp -R static static_custom
```

## Apex CLI tool: lwrite

ldwrite is a CLI tool for creating linked-deltas from your terminal.

```sh
cargo run --bin ldwrite
```

## Troubleshooting

### Compiling in MacOS:

```sh
brew install autoconf automake libtool openssl

git clone https://github.com/cyrusimap/cyrus-sasl.git
cd cyrus-sasl/
sh ./autogen.sh
make
sudo make install
cd ..
# For the next compilations, run this
OPENSSL_ROOT_DIR=/usr/local/opt/openssl cargo run --bin server
```
