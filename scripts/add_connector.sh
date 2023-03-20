#! /usr/bin/env bash
function find_prev_connector() {
    self=scripts/add_connector.sh
    git checkout $self
    cp $self $self.tmp
    # add new connector to existing list and sort it
    connectors=(aci adyen airwallex applepay authorizedotnet bambora bluesnap braintree checkout cybersource dlocal fiserv forte globalpay klarna mollie multisafepay nuvei payu rapyd shift4 stripe trustpay worldline worldpay "$1")
    IFS=$'\n' sorted=($(sort <<<"${connectors[*]}")); unset IFS
    res=`echo ${sorted[@]}`
    sed -i'' -e "s/^    connectors=.*/    connectors=($res \"\$1\")/" $self.tmp
    for i in "${!sorted[@]}"; do
    if [ "${sorted[$i]}" = "$1" ] && [ $i != "0" ]; then
        # find and return the connector name where this new connector should be added next to it 
        eval "$2='${sorted[i-1]}'"
        mv $self.tmp $self
        rm $self.tmp-e
        return 0
    fi
    done
    mv $self.tmp $self
    rm $self.tmp-e
    # if the new connector needs to be added in first place, add it after Aci, sorted order needs to be covered in code review
    eval "$2='aci'" 
}
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
prvc=''
find_prev_connector $1 prvc
prvcc="$(tr '[:lower:]' '[:upper:]' <<< ${prvc:0:1})${prvc:1}"
sed -i'' -e "s|pub mod $prvc;|pub mod $prvc;\npub mod ${pg};|" $conn.rs
sed -i'' -e "s/};/${pg}::${pgc},\n};/" $conn.rs 
sed -i'' -e "s|\"$prvc\" \(.*\)|\"$prvc\" \1\n\t\t\t\"${pg}\" => Ok(Box::new(\&connector::${pgc})),|" $src/types/api.rs
sed -i'' -e "s/pub $prvc: \(.*\)/pub $prvc: \1\n\tpub ${pg}: ConnectorParams,/" $src/configs/settings.rs
sed -i'' -e "s/$prvc.base_url \(.*\)/$prvc.base_url \1\n${pg}.base_url = \"$base_url\"/" config/Development.toml config/docker_compose.toml config/config.example.toml loadtest/config/Development.toml
sed  -r -i'' -e "s/\"$prvc\",/\"$prvc\",\n    \"${pg}\",/" config/Development.toml config/docker_compose.toml config/config.example.toml loadtest/config/Development.toml
sed -i'' -e "s/Dummy,/Dummy,\n\t${pgc},/" crates/api_models/src/enums.rs
sed -i'' -e "s/pub enum RoutableConnectors {/pub enum RoutableConnectors {\n\t${pgc},/" crates/api_models/src/enums.rs
sed -i'' -e "s/    connector::$prvcc,/    connector::$prvcc,\n\tconnector::${pgc},/" $src/core/payments/flows.rs

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
sed -i'' -e "s/    pub $prvc: \(.*\)/\tpub $prvc: \1\n\tpub ${pg}: Option<HeaderKey>,/; s/auth.toml/sample_auth.toml/" ${tests}/connector_auth.rs 
echo "\n\n[${pg}]\napi_key=\"API Key\"" >> ${tests}/sample_auth.toml

# remove temporary files created in above step
rm ${tests}/main.rs-e ${tests}/connector_auth.rs-e
cargo +nightly fmt --all
cargo check
echo "${GREEN}Successfully created connector. Running the tests of $pg.rs"

# runs tests for the new connector
cargo test --package router --test connectors -- $pg
echo "${ORANGE}Update your credentials for $pg connector in crates/router/tests/connectors/sample_auth.toml"
