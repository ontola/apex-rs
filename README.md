## Instructions

1. Copy and fill `.env.template` to `.env` before running Docker build.
2. Download and rename database SSL certificates to `certs` folder, see Dockerfile.
3. Change the cert permissions `chmod -R 700 certs`
4. Run `docker build . -t <image_name>`

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
