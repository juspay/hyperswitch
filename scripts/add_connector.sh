pg=$1;
pgc="$(tr '[:lower:]' '[:upper:]' <<< ${pg:0:1})${pg:1}"
src="crates/router/src"
conn="$src/connector"
SCRIPT="$( cd -- "$(dirname "$0")" >/dev/null 2>&1 ; pwd -P )"
if [[ -z "$pg" ]]; then 
    echo 'Connector name not present: try "sh add_connector.sh <adyen>"'
    exit
fi
cd $SCRIPT/..
rm -rf $conn/$pg $conn/$pg.rs
git checkout $conn.rs $src/types/api.rs scripts/create_connector_account.sh $src/configs/settings.rs config/Development.toml config/docker_compose.toml crates/api_models/src/enums.rs
sed -i'' -e "s/pub use self::{/pub mod ${pg};\n\npub use self::{/" $conn.rs
sed -i'' -e "s/};/${pg}::${pgc},\n};/" $conn.rs 
sed -i'' -e "s/_ => Err/\"${pg}\" => Ok(Box::new(\&connector::${pgc})),\n\t\t\t_ => Err/" $src/types/api.rs
sed -i'' -e "s/*) echo \"This connector/${pg}) required_connector=\"${pg}\";;\n\t\t*) echo \"This connector/" scripts/create_connector_account.sh
sed -i'' -e "s/pub supported: SupportedConnectors,/pub supported: SupportedConnectors,\n\tpub ${pg}: ConnectorParams,/" $src/configs/settings.rs
sed -i'' -e "s/\[scheduler\]/[connectors.${pg}]\nbase_url = \"\"\n\n[scheduler]/" config/Development.toml
sed  -r -i'' -e "s/cards = \[(.*)\]/cards = [\1, \"${pg}\"]/" config/Development.toml
sed -i'' -e "s/\[connectors.supported\]/[connectors.${pg}]\nbase_url = ""\n\n[connectors.supported]/" config/docker_compose.toml
sed  -r -i'' -e "s/cards = \[(.*)\]/cards = [\1, \"${pg}\"]/" config/docker_compose.toml
sed -i'' -e "s/Dummy,/Dummy,\n\t${pgc},/" crates/api_models/src/enums.rs
rm $conn.rs-e $src/types/api.rs-e scripts/create_connector_account.sh-e $src/configs/settings.rs-e config/Development.toml-e config/docker_compose.toml-e crates/api_models/src/enums.rs-e
cd $conn/ 
cargo install cargo-generate
cargo gen-pg $pg
mv $pg/mod.rs $pg.rs
mv $pg/test.rs ../../tests/connectors/$pg.rs
git checkout ../../tests/connectors/main.rs
sed -i'' -e "s/mod utils;/mod ${pg};\nmod utils;/" ../../tests/connectors/main.rs
rm ../../tests/connectors/main.rs-e
cargo build
echo "Successfully created connector: try running the tests of "$pg.rs