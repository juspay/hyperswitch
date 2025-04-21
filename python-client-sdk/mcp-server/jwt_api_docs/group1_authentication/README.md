# Group 1: User Authentication & Management Tests

This directory contains tests and documentation for the Hyperswitch JWT-based User Authentication and Management APIs.

## Authentication Flow Test

The `auth_flow_test.py` script tests the complete authentication flow:
1. Sign-in
2. 2FA termination and JWT token acquisition
3. Get user info
4. Update user profile
5. Sign-out

### Prerequisites

- Python 3.6 or higher
- Hyperswitch MCP modules installed
- Valid Hyperswitch user credentials

### Installation

If you haven't installed the Hyperswitch MCP modules, you can add them to your Python path:

```bash
export PYTHONPATH=$(pwd)/../..
```

### Running the Test

To run the test:

```bash
# Make the script executable
chmod +x auth_flow_test.py

# Basic usage
./auth_flow_test.py your_email@example.com your_password

# With verbose logging
./auth_flow_test.py your_email@example.com your_password -v
```

### Test Logs

The test creates a log file `auth_flow_test.log` with detailed information about each step of the authentication flow. This log is useful for troubleshooting and verifying correct implementation.

### Test Return Codes

- `0`: All tests passed successfully
- `1`: One or more tests failed

## Additional Tests

In addition to the authentication flow test, you may want to run specific tests:

### Manual Testing

You can use the Python REPL to interactively test individual components:

```python
from hyperswitch_mcp.auth import Auth
from hyperswitch_mcp.user import User

# Sign in
auth = Auth()
signin_result = auth.signin("your_email@example.com", "your_password")
totp_token = signin_result.get('token')

# Get JWT token
jwt_result = auth.terminate_2fa(totp_token, skip_two_factor_auth=True)
jwt_token = jwt_result.get('user_info_token')

# User info
user = User(jwt_token)
user_info = user.get_info()

# Sign out
signout_result = auth.signout(jwt_token)
```

## Troubleshooting

If you encounter issues, refer to the `troubleshooting.md` document in this directory for common problems and solutions. 