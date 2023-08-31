# Test Utils

The heart of `newman-dir` and `UI-tests`

## Newman Usage

- Make sure you that you've _**do not**_ have the official newman installed but rather `newman-dir`
  - `newman-dir` can be installed by running `npm install -g 'https://github.com/knutties/newman.git#feature/newman-dir'`
  - To see the features that `newman-dir` supports,  click [_**here**_](https://github.com/knutties/newman/blob/feature/newman-dir/DIR_COMMANDS.md)
- Add the connector credentials to the `connector_auth.toml` / `auth.toml`
- Export the auth file path as an environment variable:
  ```shell
  export CONNECTOR_AUTH_FILE_PATH=/path/to/auth.toml
  ```
- Run the tests:
  ```shell
  cargo run --package test_utils --bin test_utils -- --connector_name=<connector_name> --base_url=<base_url> --admin_api_key=<admin_api_key>
  ```

**Note:** You can optionally pass `--verbose` to see the logs of the tests. But make sure you that passing `--verbose` will also print the API-Keys in the logs. So, make sure you don't push the logs to any public repository. Below is an example:
```shell
cargo run --package test_utils --bin test_utils -- --connector_name=<connector_name> --base_url=<base_url> --admin_api_key=<admin_api_key> --verbose
```

### Running newman locally

Execute the following commands:
```shell
export CONNECTOR_AUTH_FILE_PATH=/path/to/auth.toml
cargo run --package test_utils --bin test_utils -- --connector_name=<connector_name> --base_url=http://127.0.0.1:8080 --admin_api_key=test_admin
# Optionally, you can add `--verbose` in the end
```
## UI tests

To run the UI tests, run the following command:
```shell
cargo test --package test_utils --test connectors -- <connector_ui_name>::<optionally_name_of_specific_function_run> --test-threads=1
```
### Example

Below is an example to run UI test to only run the `GooglePay` payment test for `adyen` connector:
```shell
cargo test --package test_utils --test connectors -- adyen_uk_ui::should_make_gpay_payment_test --test-threads=1
```

Below is an example to run all the UI tests for `adyen` connector:
```shell
cargo test --package test_utils --test connectors -- adyen_uk_ui:: --test-threads=1
```