#!/bin/sh

# Decrypt the file
mkdir $HOME/target/

mkdir $HOME/target/test
# --batch to prevent interactive command
# --yes to assume "yes" for questions

gpg --quiet --batch --yes --decrypt --passphrase="$CONNECTOR_AUTH_PASSPHRASE" \
--output $HOME/target/test/connector_auth.toml connector_auth.toml.gpg