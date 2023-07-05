sudo apt update
apt install net-tools
mkdir tests

#download connector ui tests
mv .github/testcases/ui_tests.json $HOME/target/test/connector_tests.json

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