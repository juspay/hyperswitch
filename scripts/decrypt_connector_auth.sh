#! /usr/bin/env bash

mkdir -p $HOME/target/test


# Decrypt the file
# --batch to prevent interactive command
# --yes to assume "yes" for questions
gpg --quiet --batch --yes --decrypt --passphrase="$CONNECTOR_AUTH_PASSPHRASE" \
--output $HOME/target/test/connector_auth.toml .github/secrets/connector_auth.toml.gpg