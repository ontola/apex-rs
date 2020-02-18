

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
