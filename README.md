![Apex RS Logo](./logo_title.svg)

## All Programs EXecute Rdf Source

### Getting started

1. Set up [postgres](https://www.postgresql.org/docs/current/tutorial-install.html).
1. Copy and fill `.env.template` to `.env`.
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

Running the project via docker
- `docker run -t apex-rs:latest /usr/local/bin/server` (default without arg)
- `docker run -t apex-rs:latest /usr/local/bin/importer`

### Troubleshooting

Compiling in MacOS:

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
