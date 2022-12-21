pg=$1;
pgc="$(tr '[:lower:]' '[:upper:]' <<< ${pg:0:1})${pg:1}"
if [[ -n "$pg" ]]; then
    cd ~/Desktop/orca/
    rm -rf crates/router/src/connector/$1 crates/router/src/connector/$1.rs
    git checkout crates/router/src/connector.rs crates/router/src/types/api.rs /Users/juspay/Desktop/orca/scripts/create_connector_account.sh
    sed -i '' 's/pub use self::{/pub mod '$1';\n\npub use self::{/' crates/router/src/connector.rs
    sed -i '' 's/};/'$1'::'$pgc',\n};/' crates/router/src/connector.rs 
    sed -i '' 's/_ => Err/"'$1'" => Ok(Box::new(\&connector::'$pgc')),\n\t\t\t_ => Err/' crates/router/src/types/api.rs
    sed -i '' 's/*) echo \"This connector/'$1') required_connector=\"'$1'\";;\n\t\t*) echo \"This connector/' scripts/create_connector_account.sh
    sed -i '' 's/pub supported: SupportedConnectors,/pub supported: SupportedConnectors,\n\tpub '$1': ConnectorParams,/' crates/router/src/configs/settings.rs
    
    cd crates/router/src/connector/ 
    cargo gen-pg $1
    mv $1/mod.rs $1.rs
else 
echo 'Payment gateway not present: try "sh add_connector.sh <adyen>"'
fi