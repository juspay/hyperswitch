#! /usr/bin/env bash

sudo apt-get update
sudo apt-get install net-tools
mkdir tests

COUNT=0
# Download connector ui tests
while [ ! -f $HOME/target/test/connector_tests.json ]
do
    if [ $COUNT -gt 10 ];
    then
        exit 1
    fi
    COUNT=$((COUNT+1))
    sleep 2
    wget $UI_TESTCASES_PATH && mv testcases $HOME/target/test/connector_tests.json
done

curl --retry 10 --retry-delay 2 "${UI_TESTCASES_PATH}" --output "${HOME}/target/test/connector_tests.json"

firefox --version
rm -rf $HOME/.mozilla

sh ./scripts/decrypt_browser_data.sh "$BROWSER_DATA_PASSPHRASE"

$GECKOWEBDRIVER/geckodriver > tests/geckodriver.log 2>&1 &

# Start server and redirect logs to a file
target/debug/router &

SERVER_PID=$!
# Wait for the server to start in port 8080
COUNT=0
while ! nc -z localhost 8080; do
if [ $COUNT -gt 12 ]; then # Wait for up to 2 minutes (12 * 10 seconds)
    echo "Server did not start within a reasonable time. Exiting."
    kill $SERVER_PID
    exit 1
else
    COUNT=$((COUNT+1))
    sleep 10
fi
done