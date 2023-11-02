#! /usr/bin/env bash

# Decrypt the file
# --batch to prevent interactive command
# --yes to assume "yes" for questions
gpg --quiet --batch --yes --decrypt --passphrase="$1" \
--output $HOME/browser_data.tar.gz .github/secrets/browser_data.tar.gz.gpg

# Unzip the tarball
tar xzf $HOME/browser_data.tar.gz -C $HOME
