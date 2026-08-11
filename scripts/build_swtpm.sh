#Source: https://gist.github.com/Jacobboogibear/2af9a49f8e9fdc2641ae019be58c4acd
sudo apt-get install git g++ gcc automake autoconf libtool make gcc libc-dev libssl-dev pkg-config libtasn1-6-dev libjson-glib-dev expect gawk socat libseccomp-dev -y
cd ~
git clone https://github.com/stefanberger/swtpm.git
git clone https://github.com/stefanberger/libtpms.git
cd libtpms
git checkout v0.10.0
./autogen.sh --prefix=/usr --with-tpm2 --with-openssl
make
sudo make install
cd ../swtpm
git checkout v0.9.0
./autogen.sh --prefix=/usr
make
sudo make install
cd ..
rm -rf swtpm/ libtpms/e
