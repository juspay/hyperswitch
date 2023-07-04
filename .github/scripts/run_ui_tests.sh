sudo apt update
apt install net-tools
mkdir tests

#download connector ui tests
mv .github/testcases/ui_tests.json $HOME/target/test/connector_tests.json

#install geckodriver
wget -c https://github.com/mozilla/geckodriver/releases/download/v0.33.0/geckodriver-v0.33.0-linux-aarch64.tar.gz && tar -xvzf geckodriver*
chmod +x geckodriver
mv geckodriver /usr/local/bin/
geckodriver > tests/geckodriver.log 2>&1 &

#install and run firefox
sudo add-apt-repository -y ppa:mozillateam/ppa
echo ' 
Package: * 
Pin: release o=LP-PPA-mozillateam 
Pin-Priority: 1001 
' | sudo tee /etc/apt/preferences.d/mozilla-firefox
echo 'Unattended-Upgrade::Allowed-Origins:: "LP-PPA-mozillateam:${distro_codename}";' | sudo tee /etc/apt/apt.conf.d/51unattended-upgrades-firefox
sudo apt install -y firefox
firefox

#start server and run ui tests
cargo build
target/debug/router &

#Wait for the server to start in port 8080
while netstat -lnt | awk '$4 ~ /:8080$/ {exit 1}'; do 
    if [ $SECONDS > 900 ]
    then
        exit 1
    else 
        sleep 10
    fi
done