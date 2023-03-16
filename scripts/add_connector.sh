#! /usr/bin/env bash

pg=$1;
base_url=$2;
pgc="$(tr '[:lower:]' '[:upper:]' <<< ${pg:0:1})${pg:1}"
src="crates/router/src"
conn="$src/connector"
tests="../../tests/connectors"
SCRIPT="$( cd -- "$(dirname "$0")" >/dev/null 2>&1 ; pwd -P )"
RED='\033[0;31m'
GREEN='\033[0;32m'
ORANGE='\033[0;33m'

if [ -z "$pg" ] || [ -z "$base_url" ]; then 
    echo "$RED Connector name or base_url not present: try $GREEN\"sh add_connector.sh adyen https://test.adyen.com\""
    exit
fi
cd $SCRIPT/..

# remove template files if already created for this connector
rm -rf $conn/$pg $conn/$pg.rs
git checkout $conn.rs $src/types/api.rs $src/configs/settings.rs config/Development.toml config/docker_compose.toml config/config.example.toml loadtest/config/Development.toml crates/api_models/src/enums.rs $src/core/payments/flows.rs

# add enum for this connector in required places
sed -i'' -e "s/pub use self::{/pub mod ${pg};\n\npub use self::{/" $conn.rs
sed -i'' -e "s/};/${pg}::${pgc},\n};/" $conn.rs 
sed -i'' -e "s/_ => Err/\"${pg}\" => Ok(Box::new(\&connector::${pgc})),\n\t\t\t_ => Err/" $src/types/api.rs
sed -i'' -e "s/pub supported: SupportedConnectors,/pub supported: SupportedConnectors,\n\tpub ${pg}: ConnectorParams,/" $src/configs/settings.rs
sed -i'' -e "s/\[connectors\]/[connectors]\n${pg}.base_url = \"$base_url\"/" config/Development.toml config/docker_compose.toml config/config.example.toml loadtest/config/Development.toml
sed  -r -i'' -e "s/cards = \[/cards = [\n    \"${pg}\",/" config/Development.toml config/docker_compose.toml config/config.example.toml loadtest/config/Development.toml
sed -i'' -e "s/Dummy,/Dummy,\n\t${pgc},/" crates/api_models/src/enums.rs
sed -i'' -e "s/pub enum RoutableConnectors {/pub enum RoutableConnectors {\n\t${pgc},/" crates/api_models/src/enums.rs
sed -i'' -e "s/default_imp_for_complete_authorize!(/default_imp_for_complete_authorize!(\nconnector::${pgc},/" $src/core/payments/flows.rs
sed -i'' -e "s/default_imp_for_connector_redirect_response!(/default_imp_for_connector_redirect_response!(\nconnector::${pgc},/" $src/core/payments/flows.rs

# remove temporary files created in above step
rm $conn.rs-e $src/types/api.rs-e $src/configs/settings.rs-e config/Development.toml-e config/docker_compose.toml-e config/config.example.toml-e loadtest/config/Development.toml-e crates/api_models/src/enums.rs-e $src/core/payments/flows.rs-e
cd $conn/ 

# generate template files for the connector
cargo install cargo-generate
cargo gen-pg $pg

# move sub files and test files to appropriate folder
mv $pg/mod.rs $pg.rs
mv $pg/test.rs ${tests}/$pg.rs

# remove changes from tests if already done for this connector
git checkout ${tests}/main.rs ${tests}/connector_auth.rs ${tests}/sample_auth.toml

# add enum for this connector in test folder
sed -i'' -e "s/mod utils;/mod ${pg};\nmod utils;/" ${tests}/main.rs
sed -i'' -e "s/struct ConnectorAuthentication {/struct ConnectorAuthentication {\n\tpub ${pg}: Option<HeaderKey>,/; s/auth.toml/sample_auth.toml/" ${tests}/connector_auth.rs 
echo "\n\n[${pg}]\napi_key=\"API Key\"" >> ${tests}/sample_auth.toml

# remove temporary files created in above step
rm ${tests}/main.rs-e ${tests}/connector_auth.rs-e
cargo +nightly fmt --all
cargo check
echo "${GREEN}Successfully created connector. Running the tests of $pg.rs"

# runs tests for the new connector
cargo test --package router --test connectors -- $pg
echo "${ORANGE}Update your credentials for $pg connector in crates/router/tests/connectors/sample_auth.toml"
