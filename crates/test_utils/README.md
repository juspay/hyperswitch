# Test Utils

The heart of `newman-dir` and `UI-tests`

## Usage

- Make sure you that you've **do not** have the official newman installed but rather `newman-dir`
  - `newman-dir` can be installed by running `npm install -g 'https://github.com/knutties/newman.git#feature/newman-dir'`
  ```text
  Usage: newman [options] [command]

  Options:
    -v, --version                                               output the version number
    -h, --help                                                  display help for command

  Commands:
  dir-add-folder [options] <folder-path>                      Add a folder to directory based Postman collection in the given path
  dir-add-test [options] <test-path>                          Add a test to directory based Postman collection in the given path
  dir-create [options] <collection-path>                      Create a directory based Postman collection in the given path
  dir-export [options] <postman-collection-file>              Convert a Postman collection file into its directory representation
  dir-export-import-test [options] <postman-collection-file>  Check if an export followed by import results in same collection
  dir-import [options] <collection-dir>                       Convert a Postman directory representation into a postman collection
  dir-remove-folder <folder-path>                             Remove test at given path from directory based Postman collection
  dir-remove-test <test-path>                                 Remove test at given path from directory based Postman collection
  dir-run [options] <collection-dir>                          Runs the tests in collection-dir, with all the provided options
  run [options] <collection>                                  Initiate a Postman Collection run from a given URL or path 
  ```
- Add the connector credentials to the `connector_auth.toml` / `auth.toml`
- Export the auth file path as an environment variable:
  ```shell
  export CONNECTOR_AUTH_FILE_PATH=/path/to/auth.toml
  ```
- Run the tests:
  ```shell
  cargo run --package test_utils --bin test_utils -- --connector_name=<connector_name> --base_url=<base_url> --admin_api_key=<admin_api_key>
  ```

**Note:**

You can optionally pass `--verbose` to see the logs of the tests. But make sure you that passing `--verbose` will also print the API-Keys in the logs. So, make sure you don't push the logs to any public repository.
Example:
```shell
cargo run --package test_utils --bin test_utils -- --connector_name=<connector_name> --base_url=<base_url> --admin_api_key=<admin_api_key> --verbose
```

## Running locally

```shell
export CONNECTOR_AUTH_FILE_PATH=/path/to/auth.toml
cargo run --package test_utils --bin test_utils -- --connector_name=<connector_name> --base_url=http://127.0.0.1:8080 --admin_api_key=test_admin
# Optionally, add `--verbose` in the end
```