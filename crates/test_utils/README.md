# Test Runner

The main part of running tests through `newman`.

# Usage

- Make sure you that you've the postman collection for the connector available in the `postman` dir with the name `<connector_name>.postman_collection.json`
- Add the connector credentials to the `connector_auth.toml` / `auth.toml`
- In terminal, execute:
  ```zsh
  export CONNECTOR_AUTH_FILE_PATH=/path/to/auth.toml
  cargo run --package test_utils --bin test_utils -- --connector_name=<connector_name> --base_url=<base_url> --admin_api_key=<admin_api_key>
  ```
  Optionally, `--folder_name "<name_of_folder>"` can be passed to run tests only in a specific folder.
