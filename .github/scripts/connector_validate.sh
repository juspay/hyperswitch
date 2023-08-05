#!/bin/bash
sudo apt update
apt install net-tools

wget $CONNECTORS_PATH && mv Connectors Connectors.json

# Read the JSON file content into a variable
JSON_DATA=$(cat Connectors.json)

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
        echo "$connector_name:$folder"
        fi
    done
}

# Loop through the connectors' array and add elements that are not in the IGNORE_LIST_ARRAY
FILTERED_CONNECTORS=()

for CONNECTOR in "${CONNECTORS_ARRAY[@]}"; do
    if [[ ! " ${IGNORE_LIST_ARRAY[*]} " =~ " $CONNECTOR " ]]; then
        FILTERED_CONNECTORS+=("$CONNECTOR")
    fi
done
export "${FILTERED_CONNECTORS[@]}"

for CONNECTOR in "${IGNORE_LIST_ARRAY[@]}"; do
    if [[ "$CONNECTOR" == *:* ]]; then
        IFS=':' read -r NAME FOLDER_TO_IGNORE <<< "$CONNECTOR"
        
        RESULT=$(exclude_folder "$NAME" "$FOLDER_TO_IGNORE")
    fi
    export FOLDERS="$RESULT"
done
