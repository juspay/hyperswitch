# Hyperswitch JWT API Testing Guide

This guide documents the structure, tools, and processes for testing the JWT-based API system in Hyperswitch, primarily focusing on the MCP tools and associated tests.

## Project Structure

The JWT API documentation and testing code is organized as follows:

```
/home/jarnura/github/hyperswitch/
├── jwt_api_docs/                      # Root directory for JWT API documentation
│   ├── README.md                      # Main documentation index
│   ├── auth_flow_test.py              # Main authentication flow test script (alternative)
│   ├── auth_flow_test.log             # Log file for auth flow tests
│   ├── run_auth_test.sh               # Shell script to run auth tests (alternative)
│   ├── test_tools.py                  # Primary script for testing MCP tools (uses direct imports)
│   ├── test_report.md                 # Testing results and analysis
│   ├── troubleshooting.md             # General troubleshooting guide
│   ├── jwt_api_list.md                # List of JWT APIs
│   ├── api_implementation_approach.md # Implementation approach for JWT APIs
│   │
│   ├── group1_authentication/         # Group 1: User Authentication
│   │   ├── plan.md                    # Implementation plan
│   │   ├── execution.md               # Implementation status
│   │   ├── test_results.md            # Test results
│   │   ├── auth_flow_test.py          # Group-specific test script
│   │   ├── test_auth_flow.py          # Alternative test implementation
│   │   ├── test_execution_guide.md    # Guide for running tests
│   │   └── troubleshooting.md         # Group-specific troubleshooting
│   │
│   ├── group2_business_profiles/      # Group 2: Business Profiles (Placeholder?)
│   │   ├── plan.md                    # Implementation plan
│   │   ├── execution.md               # Implementation status
│   │   └── troubleshooting.md         # Group-specific troubleshooting
│   │
│   ├── group3_api_keys/               # Group 3: API Keys Management
│   │   ├── plan.md                    # Implementation plan
│   │   ├── execution.md               # Implementation status
│   │   ├── test_api_keys.py           # API Keys testing script
│   │   ├── troubleshooting.md         # Group-specific troubleshooting
│   │   └── usage_guide.md             # Guide for using API Keys
│   │
│   └── group4_business_profiles/      # Group 4: Business Profiles (Actual?)
│       ├── plan.md                    # Implementation plan
│       ├── execution.md               # Implementation status
│       └── troubleshooting.md         # Group-specific troubleshooting
│
└── python-client-sdk/                 # Python client SDK
    └── mcp-server/                    # MCP server implementation
        ├── hyperswitch/               # Hyperswitch API client
        └── hyperswitch_mcp/           # MCP tools implementation
            ├── server.py              # MCP tool definitions
            ├── auth.py                # Authentication functions
            └── user.py                # User management functions
```

## Test Scripts

The project includes several test scripts for different testing purposes:

### 1. Primary MCP Tools Testing (Recommended)

- **`test_tools.py`**: The main script for testing the Hyperswitch MCP tools.
    - Located at: `jwt_api_docs/test_tools.py` (relative to `mcp-server` directory)
    - **Behavior:** Attempts to import and run the tools **directly** from `hyperswitch_mcp/server.py`, interacting with the live API (typically `http://localhost:8080`). It only falls back to mock implementations if direct imports fail (e.g., due to path errors).
    - **Usage:** See "Running MCP Tools Tests" section below for detailed command examples.

### 2. Alternative Authentication Flow Testing

- **`auth_flow_test.py`**: An alternative script focused on testing the complete authentication flow from sign-in to sign-out.
    - Located at: `jwt_api_docs/auth_flow_test.py`
    - Usage: `python jwt_api_docs/auth_flow_test.py --email your_email@example.com --password your_password`

- **`run_auth_test.sh`**: Shell script wrapper for running `auth_flow_test.py`.
    - Located at: `jwt_api_docs/run_auth_test.sh`
    - Usage: `./jwt_api_docs/run_auth_test.sh --email your_email@example.com --password your_password`

### 3. API Group-Specific Testing

- **`test_api_keys.py`**: Tests the API Keys Management endpoints (Group 3).
    - Located at: `jwt_api_docs/group3_api_keys/test_api_keys.py`

## Testing Environments / Modes

The primary way to test the MCP tools involves interacting with a running Hyperswitch backend (usually `http://localhost:8080`).

1.  **Direct Mode (Live API - Recommended):**
    - Tests using the actual MCP tools by running `test_tools.py`. This script imports the tool functions directly from the `hyperswitch_mcp` module.
    - Makes **actual API calls** to the configured Hyperswitch server (e.g., `localhost:8080`).
    - Validates the real interaction between the tools and the backend API.
    - Example: `python jwt_api_docs/test_tools.py --email <EMAIL> --password <PASSWORD> --test auth`

2.  **Mock Mode (Fallback):**
    - Automatically used by `test_tools.py` **only if** it fails to import the tool functions directly from `hyperswitch_mcp`.
    - Uses internal mock implementations of the tools.
    - Does **not** make actual API calls.
    - Useful for basic script logic checks if the environment is broken, but does not validate API interaction.

## Test Types

The test suite includes several types of tests:

1. **Unit Tests**: Testing individual components of the authentication system
   - Example: Testing the `say_hello` function in isolation

2. **Integration Tests**: Testing the interaction between components
   - Example: Testing the complete authentication flow

3. **End-to-End Tests**: Testing the complete user journey
   - Example: Sign-in → 2FA → Get user info → Update profile → Sign-out

## Adding New Tests

To add new tests to the project:

1. **For new API groups**:
   - Create a new directory under `jwt_api_docs/` (e.g., `groupX_new_feature/`)
   - Add implementation plan, execution document, and test scripts

2. **For new test scenarios**:
   - Add test methods to existing test scripts (like `test_tools.py`)
   - For complex scenarios, create new test scripts
   - Update the test_report.md when significant changes are made

## Running Tests

### Setting Up the Environment

1.  Navigate to the `mcp-server` directory:
    ```bash
    cd /path/to/hyperswitch/python-client-sdk/mcp-server
    ```
2.  Ensure you have the required Python packages (primarily `requests`):
    ```bash
    # Assuming you have a virtual environment set up
    pip install requests
    ```
3.  Ensure the Hyperswitch backend server is running (usually on `http://localhost:8080`).
4.  **(Optional) Python Path:** Setting `PYTHONPATH` is generally not required when running scripts from within the `mcp-server` directory. It might be needed if running scripts from a different location that need to import from `hyperswitch_mcp`.

### Running MCP Tools Tests (Recommended Method)

Run these commands from the `mcp-server` directory.

```bash
# Run all available tests (Note: reset_pwd and verify_email will fail due to known backend issues)
python jwt_api_docs/test_tools.py --email jarnura47@gmail.com --password <PASSWORD> --test all

# Run only the core auth flow
python jwt_api_docs/test_tools.py --email jarnura47@gmail.com --password <PASSWORD> --test auth

# Run password change test (will prompt for current password if not provided)
# Known working password for jarnura47@gmail.com: NewPa$$w0rd123_1745154955
python jwt_api_docs/test_tools.py --email jarnura47@gmail.com --password <CURRENT_PASSWORD> --new-password <NEW_PASSWORD> --test change_pwd

# Run password reset initiation (EXPECTED TO FAIL - Backend Issue)
# Will prompt for reset token from email (if initiation were successful) and new password
python jwt_api_docs/test_tools.py --email jarnura47@gmail.com --new-password <NEW_PASSWORD> --test reset_pwd

# Run email verification test (EXPECTED TO FAIL - Backend Issue)
# Requires valid JWT from auth flow, will prompt for verification token from email
python jwt_api_docs/test_tools.py --email jarnura47@gmail.com --password <PASSWORD> --test verify_email # Requires password to get JWT first
```
Replace `<PASSWORD>`, `<CURRENT_PASSWORD>`, `<NEW_PASSWORD>` as needed. If `--password` is omitted for flows requiring it, you will be prompted.

### Running Alternative Authentication Flow Tests

Run these commands from the `mcp-server` directory.

```bash
# Using the Python script directly
python jwt_api_docs/auth_flow_test.py --email your_email@example.com --password your_password

# Using the shell script wrapper
chmod +x jwt_api_docs/run_auth_test.sh
./jwt_api_docs/run_auth_test.sh --email your_email@example.com --password your_password
```

### Running Group-Specific Tests

```bash
# Example for API Keys (Group 3) - Run from mcp-server directory
python jwt_api_docs/group3_api_keys/test_api_keys.py
```

## Test Logging and Reporting

The test scripts generate logs and reports:

- **Logs**: Detailed logs of test execution
  - Located at: `jwt_api_docs/auth_flow_test.log`, `jwt_api_docs/test_tools.log`
  - Contains detailed information about API calls, responses, and errors

- **Reports**: Summaries of test results
  - Located at: `jwt_api_docs/test_report.md`
  - Contains tables of test results, analysis, and recommendations

## Troubleshooting Tests

If you encounter issues when running tests:

1. Check the log files (`test_tools.log`) for detailed error messages.
2. Verify that you're using valid credentials (e.g., `jarnura47@gmail.com` and the current password).
3. Check network connectivity to the API server (`http://localhost:8080`).
4. Verify Python environment and imports. Direct imports should work when running from `mcp-server`.
5. Consult the troubleshooting guides:
   - General: `jwt_api_docs/troubleshooting.md` (This guide should be updated with known issues)
   - Group-specific: `jwt_api_docs/group*/troubleshooting.md`

## API Endpoint Reference

The test scripts interact with the following API endpoints (primarily via MCP tools):

1.  **Authentication Endpoints**:
    - `POST /user/signin` - Sign in with email/password
    - `GET /user/2fa/terminate` - Terminate 2FA and get JWT token
    - `POST /user/signout` - Sign out and invalidate token
2.  **User Management Endpoints**:
    - `GET /user` - Get user information
    - `POST /user/update` - Update user profile
    - `POST /user/change_password` - Change user password
    - `POST /user/reset_password` - Initiate or confirm password reset (Known Issue: Handles both poorly)
    - `POST /user/verify/email` - Verify user email (Known Issue: 401 errors)
3.  **API Keys Management Endpoints**:
    - `/api_keys` - List, create, update, and delete API keys
    - `/api_keys/{key_id}` - Get specific API key details
4.  **Business Profiles Endpoints**:
    - Various endpoints for managing business profiles (see `business_profiles.py`)

## Known Issues

1.  **Backend API Issue (Password Reset):** The `POST /user/reset_password` endpoint incorrectly expects a `password` or `token` field during the *initiation* phase (when only email should be needed), causing 400 errors.
2.  **Backend API Issue (Email Verification):** The `POST /user/verify/email` endpoint returns a 401 Unauthorized error ("invalid JWT token") even when provided with a valid `user_info_token` obtained from the authentication flow.
3.  **Core SDK Test Failures (`pytest`):** Running `pytest` in the parent `python-client-sdk` directory shows multiple failures for tests related to Merchant Connectors and Profiles. These require investigation.

## Contributing to Testing

When contributing new tests:

1. Follow the established code structure and naming conventions
2. Add detailed comments and docstrings
3. Update the test_report.md with new test results
4. Document any issues or edge cases in the troubleshooting guides
5. Ensure backward compatibility with existing tests

## Security Considerations for Testing

When testing APIs with authentication:

1. Never commit real credentials to the repository
2. Use environment variables or command-line arguments for credentials
3. Verify that test logs don't contain sensitive information
4. Always invalidate tokens after testing (sign out)
5. Consider using dedicated test accounts instead of production accounts

## Continuous Integration

The JWT API tests can be integrated into CI/CD pipelines:

1. Run tests with mock mode for quick validation (if mock mode is needed/used)
2. Run tests with test credentials for full validation against a test server
3. Generate reports and verify test coverage

## Future Testing Improvements

(Placeholder for future ideas) 