# Group 1: User Authentication & Management - Troubleshooting Guide

This document provides solutions for common issues encountered with the JWT Authentication & User Management APIs.

## Authentication Issues

### Sign-in Problems

| Issue | Possible Causes | Solutions |
|-------|----------------|-----------|
| "Invalid credentials" error | - Incorrect email or password<br>- Account does not exist | - Verify email and password<br>- Check if the account exists<br>- Reset password if forgotten |
| "Account locked" error | - Too many failed login attempts | - Wait for the lockout period to expire<br>- Contact administrator for account unlock |
| Sign-in request hangs | - Network connectivity issues<br>- Server overload | - Check network connection<br>- Verify server status<br>- Implement request timeout and retry logic |

### 2FA Termination Issues

| Issue | Possible Causes | Solutions |
|-------|----------------|-----------|
| "Invalid TOTP token" error | - Incorrect token entered<br>- Token expired<br>- Clock synchronization issues | - Verify the token<br>- Request a new token<br>- Ensure device clock is synchronized |
| "Session expired" error | - Sign-in session timeout | - Sign in again to get a fresh TOTP token |
| JWT token not received | - Server error<br>- Network issue | - Check request format<br>- Verify network connectivity<br>- Examine server logs |

### JWT Token Issues

| Issue | Possible Causes | Solutions |
|-------|----------------|-----------|
| "Token expired" error | - JWT token has expired | - Sign in again to obtain a new token |
| "Invalid token" error | - Token has been tampered with<br>- Token format is incorrect | - Do not modify the token<br>- Ensure token is passed correctly in header |
| "Token revoked" error | - User has signed out<br>- Token has been invalidated by administrator | - Sign in again to obtain a new token |

## User Management Issues

### User Info Retrieval Issues

| Issue | Possible Causes | Solutions |
|-------|----------------|-----------|
| "Authorization failed" error | - Invalid JWT token<br>- Insufficient permissions | - Verify token validity<br>- Check user permissions |
| User info incomplete | - Profile not fully populated<br>- Data access restrictions | - Update the profile<br>- Verify access permissions |
| "User not found" error | - Account deleted<br>- Data inconsistency | - Verify account status<br>- Contact administrator |

### Profile Update Issues

| Issue | Possible Causes | Solutions |
|-------|----------------|-----------|
| "Validation error" | - Invalid data format<br>- Missing required fields | - Check data format<br>- Ensure required fields are provided |
| "Email already exists" error | - Attempting to update email to one already in use | - Choose a different email address |
| Update not reflected | - Caching issues<br>- Asynchronous processing delay | - Refresh data<br>- Wait for processing to complete |

## Sign-out Issues

| Issue | Possible Causes | Solutions |
|-------|----------------|-----------|
| Sign-out fails | - Invalid JWT token<br>- Network issues | - Check token validity<br>- Verify network connection |
| Token still valid after sign-out | - Sign-out request failed<br>- Server-side issue | - Retry sign-out<br>- Force token expiration on client side |

## API Request Issues

| Issue | Possible Causes | Solutions |
|-------|----------------|-----------|
| Rate limit exceeded | - Too many requests in a short period | - Implement rate limiting logic<br>- Reduce request frequency |
| "Bad request" error | - Malformed request<br>- Invalid parameters | - Check request format<br>- Verify parameter values |
| Server error (5xx) | - Internal server error<br>- Service unavailability | - Check server status<br>- Implement retry logic with backoff |

## Implementation Troubleshooting

### Missing JWT Bearer Token

```python
# Incorrect - Missing Authorization header
response = requests.get("https://api.hyperswitch.io/v1/user_info")

# Correct - Include JWT token in Authorization header
headers = {"Authorization": f"Bearer {jwt_token}"}
response = requests.get("https://api.hyperswitch.io/v1/user_info", headers=headers)
```

### Incorrect 2FA Termination

```python
# Incorrect - Missing TOTP token
response = requests.post("https://api.hyperswitch.io/v1/auth/terminate2fa")

# Correct - Include TOTP token in request
payload = {"totp_code": "123456"}
response = requests.post("https://api.hyperswitch.io/v1/auth/terminate2fa", json=payload)
```

### Improperly Formatted Profile Update

```python
# Incorrect - Invalid format
payload = {"name": "John Doe", "invalid_field": "value"}
response = requests.put("https://api.hyperswitch.io/v1/user", json=payload)

# Correct - Valid format
payload = {"first_name": "John", "last_name": "Doe", "company_name": "ABC Corp"}
response = requests.put("https://api.hyperswitch.io/v1/user", json=payload)
```

## Testing Environment Issues

| Issue | Possible Causes | Solutions |
|-------|----------------|-----------|
| Tests fail to run | - Missing dependencies<br>- Environment setup issues | - Install required packages<br>- Set PYTHONPATH correctly |
| Unit tests fail | - Code changes broke tests<br>- Test environment issues | - Fix code or update tests<br>- Check test environment |
| Authentication flow test fails | - Invalid credentials<br>- Server unavailability | - Verify credentials<br>- Check server status |

### Common Test Script Issues

```bash
# Issue: Permission denied when running test scripts
# Solution: Make scripts executable
chmod +x run_auth_test.sh
chmod +x run_unit_tests.sh
chmod +x run_all_tests.sh

# Issue: Module not found errors
# Solution: Set PYTHONPATH correctly
export PYTHONPATH=$(pwd)/../..

# Issue: Test credentials not accepted
# Solution: Use valid test account credentials
./run_auth_test.sh valid.email@example.com correct_password
```

## Logging and Debugging

To enable detailed logging for troubleshooting:

```python
import logging
logging.basicConfig(level=logging.DEBUG)
```

Key log files to check:
- Application logs: Check for authentication errors and request processing issues
- Server logs: Check for API endpoint availability and server errors
- Client logs: Check for request/response details and error handling

## Getting Help

If you encounter issues not covered in this guide:

1. Check the [Hyperswitch API Documentation](https://docs.hyperswitch.io)
2. Review the source code and implementation details in:
   - `python-client-sdk/mcp-server/hyperswitch_mcp/auth.py`
   - `python-client-sdk/mcp-server/hyperswitch_mcp/user.py`
3. Run the test scripts with the `-v` (verbose) flag for more detailed output
4. Contact the Hyperswitch support team for assistance

# Authentication Flow Troubleshooting Guide

This document provides solutions for common issues encountered when using the Hyperswitch JWT authentication flow.

## Sign-in Issues

### Invalid Credentials

**Symptom**: Error message `"Invalid email or password"`

**Solution**:
- Verify the email address is correct and properly formatted
- Check that the password is correct
- Reset your password if you've forgotten it

### Rate Limited

**Symptom**: Error message `"Too many sign-in attempts. Please try again later."`

**Solution**:
- Wait for 15 minutes before attempting to sign in again
- Ensure you're using the correct credentials to avoid triggering rate limits

## JWT Token Issues

### Token Not Received

**Symptom**: No JWT token received after 2FA termination

**Solution**:
- Verify that the correct TOTP token was used
- Check if 2FA is properly set up for the account
- Try again with `skip_two_factor_auth=True` if testing in a development environment

### Expired Token

**Symptom**: Error message `"Token expired"` or `"Invalid token"`

**Solution**:
- Sign in again to get a fresh token
- Check system time - if it's significantly off, tokens might appear expired
- JWT tokens typically expire after 24 hours

## User Info Issues

### Permission Denied

**Symptom**: Error message `"Permission denied"` when accessing user info

**Solution**:
- Verify the JWT token is valid and not expired
- Check that the user has the necessary permissions
- Ensure you're passing the token in the correct format: `Bearer <token>`

### Missing Profile Fields

**Symptom**: User profile data is incomplete

**Solution**:
- Newly created users might not have all profile fields populated
- Try updating the profile with the missing information
- Verify database consistency if issue persists

## Profile Update Issues

### Validation Errors

**Symptom**: Error message containing field validation errors

**Solution**:
- Check the format of each field (email, phone, etc.)
- Ensure required fields are provided
- Review length restrictions for text fields

### Concurrent Update Conflicts

**Symptom**: Error message `"Profile was updated by another request"`

**Solution**:
- Fetch the latest user profile before attempting an update
- Implement retry logic with the latest profile data
- Consider optimistic locking if updates are frequent

## Sign-out Issues

### Failed Sign-out

**Symptom**: Sign-out appears to succeed but user remains authenticated

**Solution**:
- Verify the correct JWT token was used for sign-out
- Clear client-side cookies and local storage
- Check for multiple active sessions

## Network-Related Issues

### Connection Timeouts

**Symptom**: Requests time out or fail to complete

**Solution**:
- Check network connectivity
- Verify API endpoint URLs are correct
- Increase request timeout settings for slower connections

### CORS Issues

**Symptom**: Browser console shows CORS errors

**Solution**:
- Ensure requests are made from an allowed origin
- Add required CORS headers if you control the server
- Use appropriate credentials mode for cross-origin requests

## Debugging Tips

1. **Enable Verbose Logging**: Run tests with `-v` flag for detailed logs
2. **Inspect Request/Response**: Use tools like Postman or curl with `-v` flag
3. **Check Token Contents**: Decode JWT tokens at [jwt.io](https://jwt.io) to inspect claims
4. **Server Logs**: Check server logs for backend errors
5. **Network Monitor**: Use browser developer tools to inspect network requests

## Still Stuck?

If you've tried these solutions and still experience issues:

1. Check the [Hyperswitch documentation](https://docs.hyperswitch.io)
2. Review recent changes to the authentication flow
3. Contact support with:
   - Detailed error messages
   - Steps to reproduce
   - Log output
   - Environment details (Python version, OS, etc.)

# JWT Authentication Troubleshooting Guide

This guide helps diagnose and resolve common issues with Hyperswitch JWT authentication.

## Common Authentication Errors

### 1. Invalid Credentials

**Error Message:** "Invalid email or password"

**Possible Causes:**
- Incorrect email address or password
- Account does not exist
- Account is locked due to too many failed attempts

**Solutions:**
- Double-check your email and password
- Reset your password via the forgotten password link
- Wait 30 minutes if your account is temporarily locked

### 2. JWT Token Issues

**Error Message:** "Invalid token" or "Expired token"

**Possible Causes:**
- The JWT token has expired (default expiration is 1 hour)
- The token was malformed or tampered with
- The token was issued for a different environment

**Solutions:**
- Re-authenticate to obtain a new token
- Ensure you're using the correct token format: `Bearer <token>`
- Verify the token is being sent in the Authorization header

### 3. Two-Factor Authentication Problems

**Error Message:** "2FA code required" or "Invalid 2FA code"

**Possible Causes:**
- 2FA is enabled but the code was not provided
- The provided 2FA code is incorrect or expired
- The 2FA device is not synchronized

**Solutions:**
- Ensure you're providing the 2FA code when required
- Generate a new 2FA code and try again
- Use a recovery code if you've lost access to your 2FA device

### 4. Permission Denied Errors

**Error Message:** "Permission denied" or "Insufficient privileges"

**Possible Causes:**
- Your account lacks the necessary permissions for the requested action
- The JWT token doesn't contain the required scopes or claims
- You're attempting to access a resource owned by another user

**Solutions:**
- Contact your administrator to request additional permissions
- Ensure you're using the correct account
- Verify the JWT token contains the necessary claims

## API Troubleshooting

### Request Issues

**Problem:** API requests fail with HTTP 4xx errors

**Troubleshooting Steps:**
1. Check that the Authorization header is properly formatted: `Authorization: Bearer <token>`
2. Verify the token is valid and not expired
3. Ensure you're using the correct API endpoint and HTTP method
4. Check request body format for any malformed JSON
5. Look for missing required fields in the request

### Response Issues

**Problem:** API responses contain unexpected errors or data

**Troubleshooting Steps:**
1. Examine the response status code and error message
2. Check the API documentation for expected response format
3. Verify your token has the necessary permissions
4. Look for any rate limiting or throttling messages in the response headers

## JWT Token Debugging

### Decoding JWT Tokens

To inspect the contents of a JWT token:

1. Go to [jwt.io](https://jwt.io/)
2. Paste your token in the "Encoded" section
3. Review the decoded payload to check:
   - `exp` (expiration time)
   - `iat` (issued at time)
   - User ID and claims
   - Token scopes

Do NOT share the decoded token with anyone, as it contains sensitive information.

### Token Expiration

JWT tokens have a default expiration time of 1 hour. If you need longer-lived tokens:

- Request tokens with a longer expiration time (where supported)
- Implement token refresh logic in your application
- Store the token securely and handle expiration gracefully

## Implementation Troubleshooting

### Client-Side Issues

**Problem:** Authentication works in tools like Postman but fails in your application

**Troubleshooting Steps:**
1. Compare the exact request headers between Postman and your application
2. Check for any cross-origin (CORS) issues in browser console
3. Verify you're storing and retrieving the token correctly
4. Ensure the token is not being truncated or modified

### Server-Side Issues

**Problem:** Authentication fails at the server level

**Troubleshooting Steps:**
1. Check server logs for detailed error messages
2. Verify the JWT signature verification is using the correct keys
3. Ensure the server time is synchronized (JWT validation is time-sensitive)
4. Check for any middleware conflicts or configuration issues

## Advanced Troubleshooting

### Network Analysis

For complex issues, use network analysis tools:

1. Use browser developer tools (Network tab) to inspect request/response
2. Employ tools like Charles Proxy or Wireshark for deeper analysis
3. Check for any TLS/SSL issues or certificate problems

### Getting Help

If you're still experiencing issues:

1. Collect the following information:
   - Request details (headers, body, endpoint)
   - Response details (status code, error message)
   - Timestamps of failed attempts
   - Any error logs from your application
2. Contact Hyperswitch support with these details
3. Do NOT include your full JWT token or password in support requests

## Security Best Practices

Always follow these security practices:

- Never store JWT tokens in local storage or cookies without proper security measures
- Implement token refresh mechanisms to maintain session security
- Set appropriate token expiration times based on security requirements
- Include only necessary claims in your JWT to minimize token size
- Implement proper error handling that doesn't leak sensitive information 