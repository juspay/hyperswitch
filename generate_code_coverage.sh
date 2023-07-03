#! /bin/bash

COV_PROGRAM="grcov"
INSTALL_COV_PROGRAM="cargo install grcov"
SERVER_PORT=8000
SERVER="python3"

command -v $COV_PROGRAM >/dev/null 2>&1 || { echo -e >&2 "Require \"$COV_PROGRAM\" but it's not installed.\nInstall using \"$INSTALL_COV_PROGRAM\" and restart.\nAborting."; exit 1; }
command -v $SERVER >/dev/null 2>&1 || { echo -e >&2 "Require \"$SERVER\" but it's not installed.\nAborting."; exit 1; }

export RUSTFLAGS="-Cinstrument-coverage"
export LLVM_PROFILE_FILE="$$-%p-%m.profraw"

echo "Running 'cargo test' and generating grcov reports.. This may take some time.."

cargo test && grcov . -s . -t html --branch --binary-path ./target/debug &&  rm -f $$*profraw &&  echo "starting server on localhost:$SERVER_PORT" && cd html && python3 -m http.server $SERVER_PORT
