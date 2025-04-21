# Building a Multi-Step Hyperswitch Workflow with MCP Tools

This document outlines the steps taken to implement a multi-step workflow using the Hyperswitch Python SDK and the `mcp-server` library. The goal was to create a sequence of three tools (Sign In -> Terminate 2FA -> List Business Profiles) where authentication tokens obtained in one step are used as input for the next.

## Step 1: Setting up the MCP Server Environment

1.  **Locate Server File:** The primary file for MCP tool definitions was identified as `python-client-sdk/mcp-server/hyperswitch_mcp/server.py`.
2.  **Initialize MCP:** The `FastMCP` instance was initialized within `server.py`.
3.  **Execution Environment:** Due to issues with direct execution (`python -m ...`) causing hangs or import errors, a launcher script (`run_mcp_server.sh`) was created. This script activates the correct Python virtual environment (`.venv`), sets the `PYTHONPATH` environment variable to include the parent directory (`..`), and then runs the server using `python -m hyperswitch_mcp.server`. This ensures correct module resolution and environment setup.
4.  **Testing:** The server was configured within the Cursor MCP Agent settings using the `run_mcp_server.sh` script. Direct terminal execution was found unsuitable for testing `stdio`-based MCP communication.

## Step 2: Implementing the Sign In Tool

1.  **Functionality:** A tool was needed to take an email and password and return a `totp_token` required for the next step.
2.  **Implementation (`signin_tool`):**
    *   Decorated with `@mcp.tool("Sign in to Hyperswitch")`.
    *   Utilized an existing `signin` function imported from `.auth`.
    *   Renamed the output key from `token` to `totp_token` for clarity in the sequence.
3.  **Testing:** Successfully called via the MCP interface, returning the expected `totp_token`.

## Step 3: Implementing the Terminate 2FA Tool

1.  **Functionality:** A tool was needed to take the `totp_token` (as a Bearer token) and return a `user_info_token` (JWT) containing user session details. It also needed an option to `skip_two_factor_auth`.
2.  **Initial Challenges:**
    *   The tool initially wasn't discoverable by the MCP framework. Simplifying the decorator name to `@mcp.tool("Terminate 2FA")` resolved this.
    *   Passing the boolean `skip_two_factor_auth` failed due to type mismatches. The tool was modified to accept it as a string and convert internally.
    *   Using the standard SDK methods proved difficult for direct API calls with specific auth.
    *   Initial calls to the low-level `ApiClient.call_api` failed due to incorrect parameters (`resource_path` vs `url`, extra keywords).
    *   Even with correct parameters, `call_api` returned a 200 OK but the response body appeared empty in the Python code. Investigation revealed the underlying REST client used `preload_content=False`, requiring an explicit read.
3.  **Implementation (`terminate_2fa_tool`):**
    *   Decorated with `@mcp.tool("Terminate 2FA")`.
    *   Accepts `totp_token: str` and `skip_two_factor_auth: str = "True"`.
    *   Uses `ApiClient.call_api` directly.
    *   Manually constructs the full request URL including query parameters.
    *   Manually sets the `Authorization: Bearer <totp_token>` header.
    *   Explicitly calls `response_data.read()` on the result of `call_api` to get the response body bytes.
    *   Decodes and parses the JSON body, extracting the `token` and returning it as `user_info_token`.
4.  **Testing:** After refinements, successfully called via the MCP interface using the `totp_token` from Step 2, returning the `user_info_token`.

## Step 4: Implementing the List Profiles Tool

1.  **Functionality:** A tool was needed to list business profiles for a given merchant (`account_id`), authenticated using the `user_info_token`.
2.  **Initial Challenges:**
    *   Calls using the standard SDK method (`ProfileApi.list_profiles`) configured only with the Bearer `user_info_token` resulted in 401 Unauthorized errors.
    *   Analysis of a working `curl` command and backend logs revealed that this specific endpoint required *both* the `Authorization: Bearer <user_info_token>` header *and* a standard `api-key: <merchant_api_key>` header.
    *   Attempts to configure the SDK's `Configuration` object with both keys in the `api_key` dictionary failed, suggesting the SDK couldn't apply multiple auth schemes simultaneously via this mechanism for generated methods.
3.  **Implementation (`list_business_profiles_tool`):**
    *   Decorated with `@mcp.tool("List Business Profiles (v1)")`.
    *   Accepts `user_info_token: str`, `account_id: str`, and `standard_api_key: str`.
    *   Uses `ApiClient.call_api` directly, bypassing `ProfileApi.list_profiles`.
    *   Manually constructs the request URL (`/account/{account_id}/business_profile`).
    *   Manually creates a `header_params` dictionary containing *both* the `Authorization` and `api-key` headers.
    *   Explicitly calls `response_data.read()` on the result.
    *   Decodes and parses the JSON list of profiles from the response body.
4.  **Testing:** Successfully called via the MCP interface using the `user_info_token` from Step 3 and the necessary `account_id` and `standard_api_key`, returning the list of profiles.

## Step 5: Testing the Sequence

The entire sequence was validated by calling the tools in order through the MCP interface:
1.  Called `Sign in to Hyperswitch` with email/password.
2.  Used the resulting `totp_token` as input for `Terminate 2FA`.
3.  Used the resulting `user_info_token` (along with `account_id` and `standard_api_key`) as input for `List Business Profiles (v1)`.

## Conclusion

The three-step workflow was successfully implemented as MCP tools. Key takeaways included the necessity of a proper execution environment for the MCP server, the need to sometimes bypass higher-level SDK methods in favor of direct API calls (`call_api`) for complex authentication or response handling, and the importance of explicit response body reading (`response.read()`) when dealing with libraries that might not preload content. Debugging using print statements to stderr and analyzing backend logs alongside API documentation (or working examples like `curl`) was crucial for resolving authentication and response parsing issues. 