# JWT Authentication Flow Troubleshooting Guide

This guide helps diagnose and resolve common issues with the JWT authentication flow in the Hyperswitch API.

## Common Issues

### 1. User Info Endpoint Returns 404 Error

**Symptom**: After successful sign-in and 2FA termination, the `get_user_info` call returns a 404 error with message "Unrecognized request URL" and error code `IR_02`.

**Possible Causes**:
- The API endpoint URL is incorrect (hardcoded to `http://localhost:8080/user/info`)
- The API is deployed at a different location
- The API endpoint path has changed

**Solutions**:
1. Use the `fix_user_info.py` script to find the correct API endpoint:
   ```bash
   python jwt_api_docs/fix_user_info.py --token YOUR_JWT_TOKEN
   ```
2. Update the API endpoint URL in the code:
   - Edit `hyperswitch_mcp/user.py` file
   - Change the URL from `http://localhost:8080/user/info` to the correct endpoint

### 2. JWT Token Not Recognized

**Symptom**: API calls return 401 Unauthorized or 403 Forbidden errors.

**Possible Causes**:
- The JWT token has expired
- The token doesn't have the required scopes/permissions
- The token is malformed

**Solutions**:
1. Check the token expiration:
   ```python
   import jwt
   # Decode without verification to check expiration
   token_data = jwt.decode(token, options={"verify_signature": False})
   print(f"Expiration timestamp: {token_data.get('exp')}")
   ```
2. Re-authenticate to get a fresh token
3. Verify the token has the necessary scopes (roles, permissions)

### 3. Authentication Flow Failures

**Symptom**: Authentication steps fail with various errors.

**Solutions**:
1. Verify credentials are correct
2. Check for rate limiting (too many login attempts)
3. Ensure the TOTP code is valid (for 2FA)
4. Check network connectivity to API endpoints

## Debugging Techniques

### Inspecting JWT Tokens

To decode and inspect a JWT token:

```python
import jwt
import json

def decode_jwt(token):
    try:
        # Decode without verification to inspect payload
        decoded = jwt.decode(token, options={"verify_signature": False})
        return json.dumps(decoded, indent=2)
    except Exception as e:
        return f"Error decoding token: {str(e)}"

# Example usage
token = "your.jwt.token"
print(decode_jwt(token))
```

### Testing API Endpoints Directly

Use curl to test API endpoints directly:

```bash
curl -X GET -H "Authorization: Bearer YOUR_JWT_TOKEN" -H "Accept: application/json" https://api.hyperswitch.io/user/info
```

### Enabling Verbose Logging

Add verbose logging to the test scripts by setting the appropriate flags:

```bash
python jwt_api_docs/auth_flow_test.py --email your@email.com --password "your_password" --verbose
```

## Environment Setup Issues

### API Host Configuration

The API host may be misconfigured. Check the configuration in:
- `hyperswitch_mcp/config.py`
- Environment variables (if applicable)

Ensure the host is set to the correct deployment URL:
- Development: `http://localhost:8080`
- Production: `https://api.hyperswitch.io` or similar

## API Response Codes

Common error codes and their meanings:

| Code  | Description                     | Troubleshooting                                  |
|-------|---------------------------------|--------------------------------------------------|
| IR_02 | Unrecognized request URL        | Check API endpoint URL                           |
| AU_01 | Invalid credentials             | Verify email/password                            |
| AU_02 | Token expired                   | Re-authenticate to get a fresh token             |
| AU_03 | Invalid token                   | Check token format and signing                   |
| AU_04 | Insufficient permissions        | Verify token has required scopes                 |

## Getting Help

If issues persist after trying the solutions in this guide:

1. Check the API documentation for updates
2. Review the logs in `auth_flow_test.log` for detailed error information
3. Contact Hyperswitch support with:
   - The specific error message and code
   - Steps to reproduce the issue
   - JWT token (with sensitive data redacted)
   - Log files

## Using the Fix Script

We provide a diagnostic script to find the correct API endpoint when facing 404 errors with user info retrieval:

```bash
# First sign in to get a JWT token
python jwt_api_docs/auth_flow_test.py --email your@email.com --password "your_password" --test sign_in_test

# Extract the JWT token from the logs and use it with the fix script
python jwt_api_docs/fix_user_info.py --token YOUR_JWT_TOKEN
```

The script will test various potential API endpoints and report which ones work with your token. 