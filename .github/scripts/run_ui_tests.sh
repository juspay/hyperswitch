#! /usr/bin/env bash
sudo apt update
apt install net-tools
mkdir tests

#download connector ui tests
while [ ! -f $HOME/target/test/connector_tests.json ]
do
    if [ $SECONDS > 20 ]
    then
        exit 1
    fi
    sleep 2
    wget $UI_TESTCASES_PATH && mv testcases $HOME/target/test/connector_tests.json
done

firefox --version
$GECKOWEBDRIVER/geckodriver > tests/geckodriver.log 2>&1 &

#start server and run ui tests
cargo run &

#Wait for the server to start in port 8080
while netstat -lnt | awk '$4 ~ /:8080$/ {exit 1}'; do 
    if [ $SECONDS > 900 ]
    then
        exit 1
    else 
        sleep 10
    fi
done