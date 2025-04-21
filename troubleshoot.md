# Troubleshooting MCP Server and Tool Implementation

This document lists issues encountered and their solutions during the development and testing of the Hyperswitch MCP tools (Sign In, Terminate 2FA, List Profiles) within the `python-client-sdk/mcp-server/hyperswitch_mcp/` directory.

## Issue 1: Server Hanging on Execution

*   **Problem:** Running the server script (`server.py`) via `python -m hyperswitch_mcp.server` caused the process to hang indefinitely after printing the initial startup messages, even when calling the simplest `say_hello` tool.
*   **Investigation:** Compared the execution method with the working Cursor MCP setup (which used `uv run`). Tried different `FastMCP` initialization methods. Hypothesized the issue was related to `stdio` handling or the execution environment (`PYTHONPATH`).
*   **Solution:**
    1.  Created a launcher script (`run_mcp_server.sh`) that first activates the correct virtual environment (`source .venv/bin/activate`), then sets the `PYTHONPATH` correctly (`export PYTHONPATH="$PYTHONPATH:.."`), and finally executes the server via `python -m hyperswitch_mcp.server`.
    2.  Conducted all further testing by configuring the MCP agent to use this `run_mcp_server.sh` script, rather than running the server directly in the terminal. The hanging observed in the terminal was deemed an artifact of direct `stdio` interaction.

## Issue 2: Environment and Dependency Errors

*   **Problem:** Encountered various environment issues:
    *   `ImportError: attempted relative import with no known parent package` when trying `uv run hyperswitch_mcp/server.py`.
    *   `ModuleNotFoundError` for dependencies like `python-dateutil` after environment setup.
    *   `pip` command not found within the `uv`-created virtual environment.
*   **Investigation:** Identified conflicts between system Python and virtual environments, incorrect package context when running scripts directly vs. as modules.
*   **Solution:**
    *   Used `python -m hyperswitch_mcp.server` instead of `uv run .../server.py` to provide the correct package context for relative imports.
    *   Ensured the virtual environment was consistently activated (`source .venv/bin/activate`) before running any `pip` or `python` commands.
    *   Used `python -m pip install ...` or `uv pip install ...` (while the venv was active) to ensure dependencies were installed in the correct environment.
    *   Recreated the virtual environment using `uv venv` when `pip` was missing entirely.

## Issue 3: Tool Not Found (`Terminate 2FA`)

*   **Problem:** The `terminate_2fa_tool` function, although defined with the `@mcp.tool("Terminate 2FA (Get User Info Token)")` decorator in `server.py`, was not available in the list of callable tools presented by the MCP framework.
*   **Investigation:** Verified the decorator syntax and function definition. Restarted the server multiple times. Compared with other working tools.
*   **Solution:** Simplified the decorator name to `@mcp.tool("Terminate 2FA")`. After restarting the server with the simpler name, the tool (`mcp_hyperswitch_user_based_flow_Terminate_2FA`) became available. This suggests the MCP registration mechanism might be sensitive to special characters (like parentheses) in tool names or might require a clean restart to reliably pick up changes.

## Issue 4: Incorrect Parameter Type (`skip_two_factor_auth`)

*   **Problem:** Calls to the `Terminate 2FA` tool failed with the error: "Parameter 'skip_two_factor_auth' must be of type boolean, got string". This happened even when attempting to pass boolean `True`.
*   **Investigation:** The tool definition expected a boolean, but the MCP framework interaction seemed to consistently pass it as a string.
*   **Solution:** Modified the `terminate_2fa_tool` function signature in `server.py` to accept the parameter as a string (`skip_two_factor_auth: str = "True"`) and perform the boolean conversion internally (`skip_bool = skip_two_factor_auth.lower() == 'true'`).

## Issue 5: `ApiClient.call_api` Unexpected Keyword Arguments

*   **Problem:** Direct calls using `ApiClient.call_api` in the tool implementations failed with errors like "got an unexpected keyword argument 'resource_path'" or "got an unexpected keyword argument '_return_http_data_only'".
*   **Investigation:** Read the source code for `ApiClient.call_api` in `api_client.py`. Confirmed its expected parameters were different from higher-level methods or internal helper methods (`url` vs `resource_path`, no `_return_http_data_only`, etc.).
*   **Solution:** Modified the tool code (`terminate_2fa_tool`, `list_business_profiles_tool`) to construct the full `request_url` manually and pass only the valid arguments (`method`, `url`, `header_params`) directly to `call_api`.

## Issue 6: Empty Response Body Despite 200 OK (`Terminate 2FA`)

*   **Problem:** The `Terminate 2FA` API call returned status 200 OK, but the Python code using `ApiClient.call_api` reported the response body (`response_data.data`) was empty or `None`, contradicting backend logs showing a JSON response was sent.
*   **Investigation:** Added debug logs to inspect the response object. Read the `rest.py` source code and found that the underlying `urllib3` request was always made with `preload_content=False`.
*   **Solution:** Modified the `terminate_2fa_tool` code to explicitly call `response_data.read()` on the object returned by `call_api`. This forces the reading of the response body bytes, which could then be successfully decoded and parsed.

## Issue 7: 401 Unauthorized Error (`List Profiles`)

*   **Problem:** The `List Profiles` tool consistently failed with a 401 Unauthorized error, with the backend logs indicating "Missing required param: api_key", even when the `Authorization: Bearer <user_info_token>` seemed correctly configured.
*   **Investigation:** Compared attempts with a working `curl` command which included *both* `Authorization: Bearer ...` and `api-key: ...` headers. Analyzed backend logs confirming the `api_key` parameter was mandatory for this specific endpoint. Attempts to configure both keys in the SDK's `Configuration.api_key` dictionary failed to resolve the 401, suggesting the SDK's generated method (`ProfileApi.list_profiles`) couldn't handle applying multiple authentication schemes from that dictionary correctly.
*   **Solution:** Rewritten the `list_business_profiles_tool` to completely bypass the generated `ProfileApi.list_profiles` method. Instead, it uses the low-level `ApiClient.call_api`, manually constructs the full request URL, and manually creates a `header_params` dictionary containing *both* the required `Authorization` and `api-key` headers before making the call. This direct approach successfully authenticated. 