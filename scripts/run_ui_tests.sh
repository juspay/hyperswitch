install_postgres () {
    service postgresql start &
    cargo install diesel_cli --no-default-features --features "postgres"
    export DB_USER="db_user"
    export DB_PASS="db_pass"
    export DB_NAME="hyperswitch_db"
    sudo -u postgres psql -e -c \
    "CREATE USER $DB_USER WITH PASSWORD '$DB_PASS' SUPERUSER CREATEDB CREATEROLE INHERIT LOGIN;"
    sudo -u postgres psql -e -c \
    "CREATE DATABASE $DB_NAME;"
    diesel migration --database-url postgres://db_user:db_pass@localhost:5432/hyperswitch_db run
}
install_geckodriver () {
    wget -c https://github.com/mozilla/geckodriver/releases/download/v0.33.0/geckodriver-v0.33.0-linux-aarch64.tar.gz && tar -xvzf geckodriver*
    chmod +x geckodriver
    mv geckodriver /usr/local/bin/
    geckodriver > tests/webdriver.log 2>&1
}
sudo apt update
apt install net-tools
apt-get install wget
mkdir tests

sudo apt install -y postgresql postgresql-contrib libpq-dev redis-tools redis-server

#download connector ui tests
wget https://getpantry.cloud/apiv1/pantry/17746960-b133-4fa3-a0c4-e81f717ec459/basket/testcases && mv testcases $HOME/target/test/connector_tests.json

#install and run redis
redis-server &

#install and run postgres
install_postgres > tests/postgres.log 2>&1 &

#install geckodriver
install_geckodriver &

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
cargo run > tests/server_out.log 2>&1 &
cargo test --package router --test connectors -- "stripe_ui::" --test-threads=1 >> tests/test_results.log 2>&1
cargo test --package router --test connectors -- "adyen_uk_ui::" --test-threads=1 >> tests/test_results.log 2>&1
cargo test --package router --test connectors -- "payu_ui::" --test-threads=1 >> tests/test_results.log 2>&1
cargo test --package router --test connectors -- "worldline_ui::" --test-threads=1 >> tests/test_results.log 2>&1