# Authentication Flow Test Execution Guide

This guide explains how to use the `test_auth_flow.py` script to test the complete Hyperswitch JWT authentication flow.

## Test Overview

The test script performs the following operations in sequence:

1. **Sign in to Hyperswitch** with email and password
2. **Terminate 2FA** (if enabled) and obtain a JWT token
3. **Get user information** using the JWT token
4. **Update user profile** with test data
5. **Sign out** from Hyperswitch

Each step is logged and validated for success or failure.

## Prerequisites

- Python 3.6 or higher
- Access to a Hyperswitch account with valid credentials
- The Hyperswitch MCP modules must be in your Python path

## Running the Test

### Basic Usage

```bash
python test_auth_flow.py your_email@example.com your_password
```

Replace `your_email@example.com` and `your_password` with your actual Hyperswitch credentials.

### Verbose Mode

For detailed logging that shows the full request and response data:

```bash
python test_auth_flow.py your_email@example.com your_password -v
```

## Interpreting Test Results

- The script will output each step as it executes, marking successful steps with a checkmark (✓)
- A summary is provided at the end, indicating overall success or failure
- The exit code will be 0 for success and 1 for failure (useful for CI/CD pipelines)
- A detailed log file is created at `auth_flow_test.log`

### Example Output (Success)

```
2023-08-14 15:30:45 - auth_flow_test - INFO - Starting Hyperswitch JWT Authentication Flow Test
2023-08-14 15:30:45 - auth_flow_test - INFO - Step 1: Signing in to Hyperswitch
2023-08-14 15:30:47 - auth_flow_test - INFO - ✓ Sign-in successful
2023-08-14 15:30:47 - auth_flow_test - INFO - Step 2: Terminating 2FA and obtaining JWT token
2023-08-14 15:30:49 - auth_flow_test - INFO - ✓ 2FA terminated and JWT token received
2023-08-14 15:30:49 - auth_flow_test - INFO - Step 3: Getting user info
2023-08-14 15:30:51 - auth_flow_test - INFO - ✓ Successfully retrieved user info
2023-08-14 15:30:51 - auth_flow_test - INFO - Step 4: Updating user profile
2023-08-14 15:30:53 - auth_flow_test - INFO - ✓ Successfully updated user profile
2023-08-14 15:30:53 - auth_flow_test - INFO - Step 5: Signing out
2023-08-14 15:30:54 - auth_flow_test - INFO - ✓ Successfully signed out
2023-08-14 15:30:54 - auth_flow_test - INFO - 🎉 All authentication flow tests PASSED!
```

### Example Output (Failure)

```
2023-08-14 15:35:45 - auth_flow_test - INFO - Starting Hyperswitch JWT Authentication Flow Test
2023-08-14 15:35:45 - auth_flow_test - INFO - Step 1: Signing in to Hyperswitch
2023-08-14 15:35:47 - auth_flow_test - INFO - ✓ Sign-in successful
2023-08-14 15:35:47 - auth_flow_test - INFO - Step 2: Terminating 2FA and obtaining JWT token
2023-08-14 15:35:49 - auth_flow_test - INFO - ✓ 2FA terminated and JWT token received
2023-08-14 15:35:49 - auth_flow_test - INFO - Step 3: Getting user info
2023-08-14 15:35:51 - auth_flow_test - ERROR - Failed to get user info: Error: User not authorized
2023-08-14 15:35:51 - auth_flow_test - ERROR - ❌ Some authentication flow tests FAILED. Check logs for details.
```

## Troubleshooting

If the test fails, check the following:

1. **Invalid Credentials**: Ensure your email and password are correct
2. **2FA Issues**: If your account has 2FA enabled, the script attempts to handle it automatically
3. **Network Issues**: Verify your network connection to the Hyperswitch API
4. **Permission Issues**: Ensure your account has the necessary permissions
5. **API Changes**: If the Hyperswitch API has changed, the script may need updating

For detailed debugging, check the `auth_flow_test.log` file, which contains all requests and responses when run in verbose mode.

## Security Note

This script requires your actual credentials to run. Always:
- Run the script on a secure machine
- Do not share the log files, as they may contain sensitive information
- Consider using a test account rather than your production account

## Extending the Test

You can modify the test script to add additional validation or test specific features:

- Add more profile update tests
- Test specific role-based permissions
- Validate the format and content of the JWT token
- Add cleanup steps after testing 