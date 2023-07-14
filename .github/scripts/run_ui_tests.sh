#! /usr/bin/env bash
sudo apt update
apt install net-tools
mkdir tests

COUNT=0
#download connector ui tests
while [ ! -f $HOME/target/test/connector_tests.json ]
do
    if (( $COUNT > 10 ))
    then
        exit 1
    fi
    ((COUNT++))
    sleep 2
    wget $UI_TESTCASES_PATH && mv testcases $HOME/target/test/connector_tests.json
done

firefox --version
$GECKOWEBDRIVER/geckodriver > tests/geckodriver.log 2>&1 &

#start server and run ui tests
cargo run &

COUNT=0
#Wait for the server to start in port 8080
while netstat -lnt | awk '$4 ~ /:8080$/ {exit 1}'; do 
    if (( $COUNT > 30 ))
    then
        exit 1
    else 
        ((COUNT++))
        sleep 10
    fi
done

IN="$INPUT"
for i in $(echo $IN | tr "," "\n"); do
    cargo test --package router --test connectors -- "${i}_ui::" --test-threads=1 >> tests/test_results.log 2>&1
done
cat tests/test_results.log