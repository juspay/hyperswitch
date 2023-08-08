#!/bin/bash
sudo apt update
apt install net-tools

wget $POSTMAN_CONNECTOR_PATHS && mv $POSTMAN_CONNECTOR_NAMES Connectors.json

# Read the JSON file content into a variable
JSON_DATA=$(cat Connectors.json)

RED='\033[0;31m'
RESET='\033[0m'

# Extract the connectors and ignore list values using jq
CONNECTORS=$(echo "$JSON_DATA" | jq -r '.CONNECTORS')
IGNORE_LIST=$(echo "$JSON_DATA" | jq -r '.IGNORE_LIST')

# Convert the comma-separated strings into arrays
IFS=',' read -ra CONNECTORS_ARRAY <<< "$CONNECTORS"
IFS=',' read -ra IGNORE_LIST_ARRAY <<< "$IGNORE_LIST"

function exclude_folder() {
    local connector_name="$1"
    local exclude_folder="$2"

    local collection_file="postman/$connector_name.postman_collection.json"
    
    local filter=$(jq -r --arg exclude "$exclude_folder" '
        def print_folder_names($items; $indent):
        $items[] | select(has("item")) as $folder |
        if $folder.name != $exclude then
            "\($indent)\(.name)",
            ($folder.item | print_folder_names(.; "\($indent)"))
        else
            empty
        end;

      print_folder_names(.item; "")
    ' $collection_file)

    IFS=$'\n'
    
    for folder in $filter; do
        if [ "$folder" != "Happy Cases" ] && [ "$folder" != "Variation Cases" ] && [ "$folder" != "Flow Testcases" ]; then
        if [ -n "$filtered_folders" ]; then
                filtered_folders+=",$connector_name:$folder"
            else
                filtered_folders="$connector_name:$folder"
            fi
        fi
    done

    echo "$filtered_folders"
}

# Loop through the connectors' array and add elements that are not in the IGNORE_LIST_ARRAY
FILTERED_CONNECTORS=()

for CONNECTOR in "${connectors_array[@]}"; do
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

INCLUDED_FOLDERS=()
for CONNECTOR in "${IGNORE_LIST_ARRAY[@]}"; do
    if [[ "$CONNECTOR" == *:* ]]; then
        IFS=':' read -r NAME FOLDER_TO_IGNORE <<< "$CONNECTOR"
        
        RESULT=$(exclude_folder "$NAME" "$FOLDER_TO_IGNORE")
    fi
    INCLUDED_FOLDERS+="$RESULT"
done

export FOLDERS="${INCLUDED_FOLDERS[*]}"
export CONNECTORS="${FILTERED_CONNECTORS[@]}"

if [ -n "$FOLDERS" ]; then
    echo "$FOLDERS"
fi

FAILED_CONNECTORS=()
validated_connectors=()

for i in $(echo "$FILTERED_CONNECTORS" | tr "," "\n"); do
    if [[ -n $FOLDERS && ! " ${validated_connectors[*]} " =~ " $connector " ]]; then
            for j in "$FOLDERS"; do
                IFS=':' read -r connector connector_folder <<< "$j"
                only_folders="${j//$connector:/}"
                echo "cargo run --bin test_utils -- --connector_name=$connector --base_url=$BASE_URL --admin_api_key=$ADMIN_API_KEY --folder_name=$only_folders"
                FAILED_CONNECTORS+=("$i")
            done
            validated_connectors+=("$connector")
    fi
    echo "cargo run --bin test_utils -- --connector_name=$i --base_url=$BASE_URL --admin_api_key=$ADMIN_API_KEY"
    FAILED_CONNECTORS+=("$i")
done

if [ ${#FAILED_CONNECTORS[@]} -gt 0 ]; then
    echo -e "${RED}One or more connectors failed to run:${RESET}"
    printf '%s\n' "${FAILED_CONNECTORS[@]}"
    exit 1
fi
