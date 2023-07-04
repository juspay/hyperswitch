sudo apt update
apt install net-tools
mkdir tests

#download connector ui tests
mv .github/testcases/ui_tests.json $HOME/target/test/connector_tests.json

firefox --version
sh $GECKOWEBDRIVER/geckodriver

#start server and run ui tests
cargo build
target/debug/router &

#Wait for the server to start in port 8080
while netstat -lnt | awk '$4 ~ /:8080$/ {exit 1}'; do 
    if $SECONDS -gt 900 then
        exit 1
    else 
        sleep 10; 
    fi
done