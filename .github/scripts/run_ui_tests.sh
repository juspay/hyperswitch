sudo apt update
apt install net-tools
apt-get install wget
mkdir tests

#download connector ui tests
mv .github/testcases/ui_tests.json $HOME/target/test/connector_tests.json

$GECKOWEBDRIVER > tests/geckodriver.log 2>&1 &
firefox --version

#start server and run ui tests
cargo build
target/debug/router &

#Wait for the server to start in port 8080
while netstat -lnt | awk '$4 ~ /:8080$/ {exit 1}'; do sleep 10; done