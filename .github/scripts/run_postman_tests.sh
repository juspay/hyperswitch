#! /usr/bin/env bash

mkdir -p $HOME/target/test

JSON_FILE_PATH=$HOME/target/test/Connectors.json

# Fetches the Connector's list and stores it in Connectors.json file
wget -q --output-document "$JSON_FILE_PATH" "$POSTMAN_CONNECTOR_PATHS"

# Read the JSON file content into a variable
JSON_DATA=$(cat "$JSON_FILE_PATH")
# Extract the connectors and ignore_list values using jq
connectors=$(echo "$JSON_DATA" | jq -r '.connectors | @tsv')
ignore_list=$(echo "$JSON_DATA" | jq -r '.ignore_list | @tsv')

# Convert the comma-separated strings into arrays
IFS=$'\t' read -ra CONNECTORS_ARRAY <<< "$connectors"
IFS=$'\t' read -ra IGNORE_LIST_ARRAY <<< "$ignore_list"
IFS=$'\n'

# Colors for the output
RED='\033[0;31m'
RESET='\033[0m'

# This function will exclude the folders since we don't have a way to exclude folders in the newman
function exclude_folder() {
    local connector_name="$1"
    local exclude_folder="$2"
    # Reads the collection
    local collection_file="postman/$connector_name.postman_collection.json"

    # Fetches the folder names and excludes the folder that is passed as an argument
    local filter=$(jq -r --arg exclude "$exclude_folder" '
        def print_folder_names($items; $indent):
        $items[] | select(has("item")) as $folder |
        if ($exclude | split(",") | index($folder.name) == null) then
            "\($indent)\(.name)",
            ($folder.item | print_folder_names(.; "\($indent)"))
        else
            empty
        end;

      print_folder_names(.item; "")
    ' --arg exclude "$exclude_folder" $collection_file)

    # Loop through the folders and exclude the folders Main folders since we want only the folders that contains tests
    for folder in $filter; do
        if [ "$folder" != "Happy Cases" ] && [ "$folder" != "Variation Cases" ] && [ "$folder" != "Flow Testcases" ]; then
            if [ -n "$filtered_folders" ]; then
                filtered_folders+=",$connector_name:\"$folder\""
            else
                filtered_folders="$connector_name:\"$folder\""
            fi
        fi
    done

    # Returns the folders that contains tests
    echo "$filtered_folders"
}

FILTERED_CONNECTORS=()

# Loop through the connectors' array and add elements that are not in the IGNORE_LIST_ARRAY
for CONNECTOR in "${CONNECTORS_ARRAY[@]}"; do
    ignore=false
    for IGNORED_CONNECTOR in "${IGNORE_LIST_ARRAY[@]}"; do
        if [[ "$CONNECTOR" == "$IGNORED_CONNECTOR" ]]; then
            ignore=true
            break
        fi
    done

    if [[ "$ignore" == "false" ]]; then
        if [[ -n "$filtered_connectors" ]]; then
            FILTERED_CONNECTORS+=",$CONNECTOR"
        else
            FILTERED_CONNECTORS="$CONNECTOR"
        fi
    fi
done

export CONNECTORS="${FILTERED_CONNECTORS[@]}"

FAILED_CONNECTORS=()
validated_connectors=()

# Loop through the CONNECTORS_ARRAY and run the tests
for i in $(echo "$FILTERED_CONNECTORS" | tr "," "\n"); do
    run_connector=false

    # Loop through the IGNORE_LIST_ARRAY and exclude the folders that are not needed, run the test for the rest of the folders
    for CONNECTOR in "${IGNORE_LIST_ARRAY[@]}"; do
        if [[ "$CONNECTOR" == *:* ]]; then
            IFS=':' read -r name folder_to_ignore <<< "$CONNECTOR"
            included_folders="$(exclude_folder "$name" "$folder_to_ignore")"

            for j in "$included_folders"; do
                IFS=':' read -r connector connector_folder <<< "$j"
                only_folders="${j//$connector:/}"
                if [[ "$CONNECTOR" == "$i" && ! " ${validated_connectors[*]} " =~ " $CONNECTOR " ]]; then
                    echo "cargo run --bin test_utils -- --connector_name=$i --base_url=$BASE_URL --admin_api_key=$ADMIN_API_KEY --folder_name=$only_folders"
                    run_connector=true
                    # if ! cargo run --bin test_utils -- --connector_name="$i" --base_url="$BASE_URL" --admin_api_key="$ADMIN_API_KEY" --folder_name="$only_folders"; then
                    #     FAILED_CONNECTORS+=("$i")
                    #     run_connector=true
                    # fi
                fi
                validated_connectors+=("$CONNECTOR")
            done
        fi
    done
    
    # Run the tests for the rest
    if [[ "$run_connector" == "false" ]]; then
        if [[ ! " ${validated_connectors[*]} " =~ " $i " ]]; then
            echo "cargo run --bin test_utils -- --connector_name=$i --base_url=$BASE_URL --admin_api_key=$ADMIN_API_KEY"
            # if ! cargo run --bin test_utils -- --connector_name="$i" --base_url="$BASE_URL" --admin_api_key="$ADMIN_API_KEY"; then
            #   FAILED_CONNECTORS+=("$i")
            # fi
            # validated_connectors+=("$i")
        fi
    fi
done

# If there are failed connectors, print them and exit with error
if [ ${#FAILED_CONNECTORS[@]} -gt 0 ]; then
    echo -e "${RED}One or more connectors failed to run:${RESET}"
    printf '%s\n' "${FAILED_CONNECTORS[@]}"
    exit 1
fi