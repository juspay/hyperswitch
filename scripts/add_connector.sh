pg=$1;
pgc="$(tr '[:lower:]' '[:upper:]' <<< ${pg:0:1})${pg:1}"
src="crates/router/src"
conn="$src/connector"
tests="../../tests/connectors/"
SCRIPT="$( cd -- "$(dirname "$0")" >/dev/null 2>&1 ; pwd -P )"
if [[ -z "$pg" ]]; then 
    echo 'Connector name not present: try "sh add_connector.sh <adyen>"'
    exit
fi
cd $SCRIPT/..
# remove template files if already created for this connector
rm -rf $conn/$pg $conn/$pg.rs
git checkout $conn.rs $src/types/api.rs $src/configs/settings.rs config/Development.toml config/docker_compose.toml config/config.example.toml loadtest/config/Development.toml crates/api_models/src/enums.rs
# add enum for this connector in required places
sed -i'' -e "s/pub use self::{/pub mod ${pg};\n\npub use self::{/" $conn.rs
sed -i'' -e "s/};/${pg}::${pgc},\n};/" $conn.rs 
sed -i'' -e "s/_ => Err/\"${pg}\" => Ok(Box::new(\&connector::${pgc})),\n\t\t\t_ => Err/" $src/types/api.rs
sed -i'' -e "s/pub supported: SupportedConnectors,/pub supported: SupportedConnectors,\n\tpub ${pg}: ConnectorParams,/" $src/configs/settings.rs
sed -i'' -e "s/\[scheduler\]/[connectors.${pg}]\nbase_url = \"\"\n\n[scheduler]/" config/Development.toml
sed  -r -i'' -e "s/cards = \[(.*)\]/cards = [\1, \"${pg}\"]/" config/Development.toml
sed -i'' -e "s/\[connectors.supported\]/[connectors.${pg}]\nbase_url = ""\n\n[connectors.supported]/" config/docker_compose.toml
sed  -r -i'' -e "s/cards = \[(.*)\]/cards = [\1, \"${pg}\"]/" config/docker_compose.toml
sed -i'' -e "s/\[connectors.supported\]/[connectors.${pg}]\nbase_url = ""\n\n[connectors.supported]/" config/config.example.toml
sed  -r -i'' -e "s/cards = \[(.*)\]/cards = [\1, \"${pg}\"]/" config/config.example.toml
sed -i'' -e "s/\[connectors.supported\]/[connectors.${pg}]\nbase_url = ""\n\n[connectors.supported]/" loadtest/config/Development.toml
sed  -r -i'' -e "s/cards = \[(.*)\]/cards = [\1, \"${pg}\"]/" loadtest/config/Development.toml
sed -i'' -e "s/Dummy,/Dummy,\n\t${pgc},/" crates/api_models/src/enums.rs
sed -i'' -e "s/pub enum RoutableConnectors {/pub enum RoutableConnectors {\n\t${pgc},/" crates/api_models/src/enums.rs
# remove temporary files created in above step
rm $conn.rs-e $src/types/api.rs-e $src/configs/settings.rs-e config/Development.toml-e config/docker_compose.toml-e config/config.example.toml-e loadtest/config/Development.toml-e crates/api_models/src/enums.rs-e
cd $conn/ 
# generate template files for the connector
cargo install cargo-generate
cargo gen-pg $pg
# move sub files and test files to appropriate folder
mv $pg/mod.rs $pg.rs
mv $pg/test.rs ${tests}/$pg.rs
# remove changes from tests if already done for this connector
git checkout ${tests}/main.rs ${tests}/connector_auth.rs 
# add enum for this connector in test folder
sed -i'' -e "s/mod utils;/mod ${pg};\nmod utils;/" ${tests}/main.rs
sed -i'' -e "s/struct ConnectorAuthentication {/struct ConnectorAuthentication {\n\tpub ${pg}: Option<HeaderKey>,/" ${tests}/connector_auth.rs 
# remove temporary files created in above step
rm ${tests}/main.rs-e ${tests}/connector_auth.rs-e 
cargo build
echo "Successfully created connector. Running the tests of "$pg.rs
# runs tests for the new connector
cargo test --package router --test connectors -- $pg