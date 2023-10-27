# Test Utils

The heart of `newman`(with directory support) and `UI-tests`

> If you're developing a collection and you want to learn more about it, click [_**here**_](/README.md)

## Newman

- Make sure you that you _**do not**_ have the newman (from the Postman team) installed but rather the `newman` fork with directory support
- The `newman` fork can be installed by running `npm install -g 'https://github.com/knutties/newman.git#feature/newman-dir'`
- To see the features that the fork of `newman` supports, click [_**here**_](https://github.com/knutties/newman/blob/feature/newman-dir/DIR_COMMANDS.md)

## Test Utils Usage

- Add the connector credentials to the `connector_auth.toml` / `auth.toml` by creating a copy of the `sample_auth.toml` from `router/tests/connectors/sample_auth.toml`
- Export the auth file path as an environment variable:
  ```shell
  export CONNECTOR_AUTH_FILE_PATH=/path/to/auth.toml
  ```

### Supported Commands

Required fields:

- `--admin_api_key` -- Admin API Key of the environment. `test_admin` is the Admin API Key for running locally
- `--base_url` -- Base URL of the environment. `http://127.0.0.1:8080` / `http://localhost:8080` is the Base URL for running locally
- `--connector_name` -- Name of the connector that you wish to run. Example: `adyen`, `shift4`, `stripe`

Optional fields:

- `--folder` -- To run individual folders in the collection
  - Use double quotes to specify folder name. If you wish to run multiple folders, separate them with a comma (`,`)
  - Example: `--folder "QuickStart"` or `--folder "Health check,QuickStart"`
- `--verbose` -- A boolean to print detailed logs (requests and responses)

**Note:** Passing `--verbose` will also print the connector as well as admin API keys in the logs. So, make sure you don't push the commands with `--verbose` to any public repository.

### Running tests

- Tests can be run with the following command:
  ```shell
  cargo run --package test_utils --bin test_utils -- --connector_name=<connector_name> --base_url=<base_url> --admin_api_key=<admin_api_key> \
  # optionally
  --folder "<folder_name_1>,<folder_name_2>,...<folder_name_n>" --verbose
  ```

**Note**: You can omit `--package test_utils` at the time of running the above command since it is optional.

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
