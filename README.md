![Apex RS Logo](./logo_title.svg)

## All Programs EXecute Rdf Source

### Getting started
1. Copy and fill `.env.template` to `.env`.
3. When using SSL with postgres
    2. Download and rename database SSL certificates to `certs` folder, see Dockerfile.
    3. Change the cert permissions `chmod -R 700 certs`
2. Run `cargo install diesel-cli` and run `diesel setup` to initialize the db schema.


Building the project manually
- `cargo build`
Building the project via docker
- `DOCKER_BUILDKIT=1 docker build . -t apex-rs:latest`

Running the project manually
- `cargo run . --bin server`
- `cargo run . --bin importer`

Running the project via docker
- `docker run -t apex-rs:latest /usr/local/bin/server` (default without arg)
- `docker run -t apex-rs:latest /usr/local/bin/importer`

### osx
For compiling
```
brew install autoconf automake libtool openssl

git clone https://github.com/cyrusimap/cyrus-sasl.git
cd cyrus-sasl/
sh ./autogen.sh
make
sudo make install

OPENSSL_ROOT_DIR=/usr/local/opt/openssl cargo run
```
