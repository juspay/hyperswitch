sudo apt update
apt install net-tools
apt-get install wget
mkdir tests

#download connector ui tests
wget --wait 10 --random-wait --continue $UI_TESTCASES_PATH && mv testcases $HOME/target/test/connector_tests.json

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
cargo run &

#Wait for the server to start in port 8080
while netstat -lnt | awk '$4 ~ /:8080$/ {exit 1}'; do sleep 10; done