#! /usr/bin/env bash

function find_prev_connector() {
    self=scripts/add_connector.sh
    # Comment below line to stop undoing changes when the script is triggered, make sure you undo this change before pushing
    git checkout $self
    cp $self $self.tmp
    # Add new connector to existing list and sort it
    connectors=(aci adyen adyenplatform affirm airwallex amazonpay applepay archipel authipay authorizedotnet bambora bamboraapac bankofamerica barclaycard billwerk bitpay blackhawknetwork bluesnap boku braintree breadpay calida cashtocode celero chargebee checkbook checkout coinbase cryptopay ctp_visa custombilling cybersource datatrans deutschebank digitalvirgo dlocal dummyconnector dwolla ebanx elavon envoy facilitapay finix fiserv fiservemea fiuu flexiti forte getnet gigadat globalpay globepay gocardless gpayments helcim hipay hyperswitch_vault hyperwallet hyperwallet iatapay inespay itaubank jpmorgan juspaythreedsserver katapult klarna loonio mifinity mollie moneris mpgs multisafepay netcetera nexinets nexixpay nomupay noon nordea novalnet nuvei opayo opennode paybox payeezy payjustnow payload payme payone paypal paysafe paystack paytm payu peachpayments phonepe placetopay plaid powertranz prophetpay rapyd razorpay recurly redsys santander shift4 sift silverflow square stax stripe stripebilling taxjar tesouro threedsecureio thunes tokenex tokenio trustpay trustpayments tsys unified_authentication_service vgs volt wellsfargo wellsfargopayout wise worldline worldpay worldpayvantiv worldpayxml xendit zift zsl "$1")


    IFS=$'\n' sorted=($(sort <<<"${connectors[*]}")); unset IFS
    res="$(echo ${sorted[@]})"
    sed -i'' -e "s/^    connectors=.*/    connectors=($res \"\$1\")/" $self.tmp
    for i in "${!sorted[@]}"; do
    if [ "${sorted[$i]}" = "$1" ] && [ $i != "0" ]; then
        # Find and return the connector name where this new connector should be added next to it
        eval "$2='${sorted[i-1]}'"
        mv $self.tmp $self
        rm $self.tmp-e
        return 0
    fi
    done
    mv $self.tmp $self
    rm $self.tmp-e
    # If the new connector needs to be added in first place, add it after Aci, sorted order needs to be covered in code review
    eval "$2='aci'"
}

payment_gateway=$(echo $1 | tr '[:upper:]' '[:lower:]')
base_url=$2;
payment_gateway_camelcase="$(tr '[:lower:]' '[:upper:]' <<< ${payment_gateway:0:1})${payment_gateway:1}"
src="crates/router/src"
conn="crates/hyperswitch_connectors/src/connectors"
tests="../../tests/connectors"
test_utils="../../../test_utils/src"
SCRIPT="$( cd -- "$(dirname "$0")" >/dev/null 2>&1 ; pwd -P )"
RED='\033[0;31m'
GREEN='\033[0;32m'
ORANGE='\033[0;33m'

if [ -z "$payment_gateway" ] || [ -z "$base_url" ]; then
    echo "$RED Connector name or base_url not present: try $GREEN\"sh add_connector.sh adyen https://test.adyen.com\""
    exit
fi
cd $SCRIPT/..

# Remove template files if already created for this connector
rm -rf $conn/$payment_gateway $conn/$payment_gateway.rs
git checkout $conn.rs $src/types/api/connector_mapping.rs $src/configs/settings.rs config/development.toml config/docker_compose.toml config/config.example.toml loadtest/config/development.toml crates/api_models/src/connector_enums.rs crates/euclid/src/enums.rs crates/api_models/src/routing.rs $src/core/payments/flows.rs crates/common_enums/src/connector_enums.rs crates/common_enums/src/connector_enums.rs-e $src/types/connector_transformers.rs $src/core/admin.rs

# Add enum for this connector in required places
previous_connector=''
find_prev_connector $payment_gateway previous_connector
previous_connector_camelcase="$(tr '[:lower:]' '[:upper:]' <<< ${previous_connector:0:1})${previous_connector:1}"
sed -i'' -e "s|pub mod $previous_connector;|pub mod $previous_connector;\npub mod ${payment_gateway};|" $conn.rs
sed -i'' -e "s/};/ ${payment_gateway}::${payment_gateway_camelcase},\n};/" $conn.rs
sed -i'' -e "/pub use hyperswitch_connectors::connectors::{/ s/{/{\n    ${payment_gateway}, ${payment_gateway}::${payment_gateway_camelcase},/" $src/connector.rs
sed -i'' -e "s|$previous_connector_camelcase \(.*\)|$previous_connector_camelcase \1\n\t\t\tRoutableConnectors::${payment_gateway_camelcase} => euclid_enums::Connector::${payment_gateway_camelcase},|" crates/api_models/src/routing.rs
sed -i'' -e "s/pub $previous_connector: \(.*\)/pub $previous_connector: \1\n\tpub ${payment_gateway}: ConnectorParams,/" crates/hyperswitch_interfaces/src/configs.rs
sed -i'' -e "s|$previous_connector.base_url \(.*\)|$previous_connector.base_url \1\n${payment_gateway}.base_url = \"$base_url\"|" config/development.toml config/docker_compose.toml config/config.example.toml loadtest/config/development.toml config/deployments/integration_test.toml config/deployments/production.toml config/deployments/sandbox.toml
sed -i '' -e "s/\(pub enum Connector {\)/\1\n\t${payment_gateway_camelcase},/" crates/api_models/src/connector_enums.rs
sed -i '' -e "/\/\/ Add Separate authentication support for connectors/{N;s/\(.*\)\n/\1\n\t\t\t| Self::${payment_gateway_camelcase}\n/;}" crates/api_models/src/connector_enums.rs
sed -i '' -e "s/\(match connector_name {\)/\1\n\t\tapi_enums::Connector::${payment_gateway_camelcase} => {${payment_gateway}::transformers::${payment_gateway_camelcase}AuthType::try_from(val)?;Ok(())}/" $src/core/admin.rs
sed -i '' -e "s/\(pub enum Connector {\)/\1\n\t${payment_gateway_camelcase},/" crates/euclid/src/enums.rs

default_impl_files=(
  "crates/hyperswitch_connectors/src/default_implementations.rs"
  "crates/hyperswitch_connectors/src/default_implementations_v2.rs"
)

# Inserts the new connector into macro blocks in default_implementations files.
# - If previous_connector exists in a macro, new_connector is added after it (maintaining logical order).
# - If previous_connector is missing, new_connector is added at the top of the macro block.
# - Ensures no duplicate entries and handles all default_imp macro variants.
# Iterate through all files where default implementations are defined
for file in "${default_impl_files[@]}"; do
  tmpfile="${file}.tmp"

  # Use AWK to parse and update macro blocks for connector registration
  awk -v prev="$previous_connector_camelcase" -v new="$payment_gateway_camelcase" '
  BEGIN { in_macro = 0 }

  {
    if ($0 ~ /^default_imp_for_.*!\s*[\({]$/) {
      in_macro = 1
      inserted = 0
      found_prev = 0
      found_new = 0
      macro_lines_count = 0
      delete macro_lines

      macro_header = $0
      macro_open = ($0 ~ /\{$/) ? "{" : "("
      macro_close = (macro_open == "{") ? "}" : ");"
      next
    }

    if (in_macro) {
      if ((macro_close == "}" && $0 ~ /^[[:space:]]*}[[:space:]]*$/) ||
          (macro_close == ");" && $0 ~ /^[[:space:]]*\);[[:space:]]*$/)) {

        for (i = 1; i <= macro_lines_count; i++) {
          line = macro_lines[i]
          clean = line
          gsub(/^[ \t]+/, "", clean)
          gsub(/[ \t]+$/, "", clean)
          if (clean == "connectors::" prev ",") found_prev = 1
          if (clean == "connectors::" new ",") found_new = 1
        }

        print macro_header

        if (!found_prev && !found_new) {
          print "    connectors::" new ","
          inserted = 1
        }

        for (i = 1; i <= macro_lines_count; i++) {
          line = macro_lines[i]
          clean = line
          gsub(/^[ \t]+/, "", clean)
          gsub(/[ \t]+$/, "", clean)

          print "    " clean

          if (!inserted && clean == "connectors::" prev ",") {
            if (!found_new) {
              print "    connectors::" new ","
              inserted = 1
            }
          }
        }

        print $0
        in_macro = 0
        next
      }

      macro_lines[++macro_lines_count] = $0
      next
    }

    print $0
  }' "$file" > "$tmpfile" && mv "$tmpfile" "$file"
done


sed -i '' -e "\$a\\
\\
[${payment_gateway}]\\
[${payment_gateway}.connector_auth.HeaderKey]\\
api_key = \\\"API Key\\\"" crates/connector_configs/toml/sandbox.toml

sed -i '' -e "\$a\\
\\
[${payment_gateway}]\\
[${payment_gateway}.connector_auth.HeaderKey]\\
api_key = \\\"API Key\\\"" crates/connector_configs/toml/development.toml

sed -i '' -e "\$a\\
\\
[${payment_gateway}]\\
[${payment_gateway}.connector_auth.HeaderKey]\\
api_key = \\\"API Key\\\"" crates/connector_configs/toml/production.toml

sed -i'' -e "s/^default_imp_for_connector_request_id!(/default_imp_for_connector_request_id!(\n    connectors::${payment_gateway_camelcase},/" $src/core/payments/flows.rs
sed -i'' -e "s/^default_imp_for_fraud_check!(/default_imp_for_fraud_check!(\n    connectors::${payment_gateway_camelcase},/" $src/core/payments/flows.rs
sed -i'' -e "s/^default_imp_for_connector_authentication!(/default_imp_for_connector_authentication!(\n    connectors::${payment_gateway_camelcase},/" $src/core/payments/flows.rs
sed -i'' -e "/pub ${previous_connector}: Option<ConnectorTomlConfig>,/a\\
    pub ${payment_gateway}: Option<ConnectorTomlConfig>,
" crates/connector_configs/src/connector.rs

sed -i'' -e "/mod utils;/ s/mod utils;/mod ${payment_gateway};\nmod utils;/" crates/router/tests/connectors/main.rs
sed -i'' -e "s/^default_imp_for_new_connector_integration_payouts!(/default_imp_for_new_connector_integration_payouts!(\n    connector::${payment_gateway_camelcase},/" crates/router/src/core/payments/connector_integration_v2_impls.rs
sed -i'' -e "s/^default_imp_for_new_connector_integration_frm!(/default_imp_for_new_connector_integration_frm!(\n    connector::${payment_gateway_camelcase},/" crates/router/src/core/payments/connector_integration_v2_impls.rs
sed -i'' -e "s/^default_imp_for_new_connector_integration_connector_authentication!(/default_imp_for_new_connector_integration_connector_authentication!(\n    connector::${payment_gateway_camelcase},/" crates/router/src/core/payments/connector_integration_v2_impls.rs

sed -i'' -e "/pub ${previous_connector}: ConnectorParams,/a\\
    pub ${payment_gateway}: ConnectorParams,
" crates/hyperswitch_domain_models/src/connector_endpoints.rs


# Remove temporary files created in above step
rm $conn.rs-e $src/types/api/connector_mapping.rs-e $src/configs/settings.rs-e config/development.toml-e config/docker_compose.toml-e config/config.example.toml-e loadtest/config/development.toml-e crates/api_models/src/connector_enums.rs-e crates/euclid/src/enums.rs-e crates/api_models/src/routing.rs-e $src/core/payments/flows.rs-e crates/common_enums/src/connector_enums.rs-e $src/types/connector_transformers.rs-e $src/core/admin.rs-e crates/hyperswitch_connectors/src/default_implementations.rs-e crates/hyperswitch_connectors/src/default_implementations_v2.rs-e crates/hyperswitch_interfaces/src/configs.rs-e $src/connector.rs-e config/deployments/integration_test.toml-e config/deployments/production.toml-e config/deployments/sandbox.toml-e temp crates/connector_configs/src/connector.rs-e crates/router/tests/connectors/main.rs-e crates/router/src/core/payments/connector_integration_v2_impls.rs-e crates/hyperswitch_domain_models/src/connector_endpoints.rs-e

cd $conn/

# Generate template files for the connector
cargo install cargo-generate
cargo generate --path ../../../../connector-template -n $payment_gateway

# Move sub files and test files to appropriate folder
mv $payment_gateway/mod.rs $payment_gateway.rs
mkdir -p ../../../router/tests/connectors
mv "$payment_gateway/test.rs" ../../../router/tests/connectors/$payment_gateway.rs


# Remove changes from tests if already done for this connector
git checkout ${tests}/main.rs ${test_utils}/connector_auth.rs ${tests}/sample_auth.toml

# Add enum for this connector in test folder
sed -i'' -e "s/mod utils;/mod ${payment_gateway};\nmod utils;/" ${tests}/main.rs
sed -i'' -e "s/    pub $previous_connector: \(.*\)/\tpub $previous_connector: \1\n\tpub ${payment_gateway}: Option<HeaderKey>,/" ${test_utils}/connector_auth.rs
echo "\n\n[${payment_gateway}]\napi_key=\"API Key\"" >> ${tests}/sample_auth.toml

# Remove temporary files created in above step
rm ${tests}/main.rs-e ${test_utils}/connector_auth.rs-e
cargo +nightly fmt --all
cargo check --features v1
echo "${GREEN}Successfully created connector. Running the tests of $payment_gateway.rs"

# Runs tests for the new connector
cargo test --package router --test connectors -- $payment_gateway
echo "${ORANGE}Update your credentials for $payment_gateway connector in crates/router/tests/connectors/sample_auth.toml"
