# Troubleshooting Guide: JWT Authentication in Hyperswitch

This guide helps you diagnose and resolve common issues with JWT-based authentication in the Hyperswitch platform.

## Common Issues and Solutions

### Sign-In Issues

#### Issue: Unable to Sign In

**Symptoms:**
- Error message: "Invalid credentials"
- HTTP 401 Unauthorized response
- No JWT token received

**Possible Causes and Solutions:**

1. **Incorrect Email or Password**
   - Double-check credentials
   - Reset password if necessary
   - Check for space characters before or after credentials

2. **Account Locked**
   - After multiple failed attempts, accounts may be temporarily locked
   - Wait 30 minutes and try again
   - Contact administrator if the issue persists

3. **Network Issues**
   - Verify your network connection
   - Check if you can access other secure services
   - Try a different network connection

#### Issue: 2FA Problems

**Symptoms:**
- Error message: "Invalid 2FA code"
- HTTP 401 Unauthorized response
- Cannot proceed past 2FA step

**Possible Causes and Solutions:**

1. **Incorrect TOTP Code**
   - Ensure your device time is synchronized
   - Use a freshly generated code (TOTP codes expire quickly)
   - Double-check code entry

2. **2FA App Issues**
   - Re-scan the QR code if possible
   - Verify the app is working correctly with other services
   - Consider using backup codes if available

3. **2FA Not Properly Set Up**
   - Contact administrator to verify 2FA is properly configured
   - You may need to re-enroll in 2FA

### JWT Token Issues

#### Issue: JWT Token Expired

**Symptoms:**
- Error message: "Token expired"
- HTTP 401 Unauthorized response
- Sudden logout during active session

**Possible Causes and Solutions:**

1. **Normal Expiration**
   - JWT tokens have a limited lifetime (typically 1 hour)
   - Sign in again to receive a new token
   - For extended operations, consider implementing refresh tokens

2. **Clock Skew**
   - Server and client time differences can cause premature expiration
   - Synchronize your device clock with an NTP server

#### Issue: Invalid JWT Token

**Symptoms:**
- Error message: "Invalid token"
- HTTP 401 Unauthorized response
- Unable to access protected resources

**Possible Causes and Solutions:**

1. **Token Tampering**
   - Do not manually modify JWT tokens
   - Ensure you're using the exact token received during authentication

2. **Wrong Token Format**
   - Ensure the Authorization header uses the format: `Bearer <token>`
   - Do not include additional characters or spaces

3. **Token Revoked**
   - The token may have been revoked for security reasons
   - Sign in again to receive a new token

### User Profile Issues

#### Issue: Unable to Retrieve User Info

**Symptoms:**
- Error message: "Unauthorized" when fetching user info
- HTTP 401 Unauthorized response
- Empty or partial user data

**Possible Causes and Solutions:**

1. **Insufficient Permissions**
   - Your account may not have permission to access this information
   - Contact administrator to review account permissions

2. **Invalid JWT Token**
   - See the "Invalid JWT Token" section above
   - Sign in again to receive a new token

#### Issue: Cannot Update Profile

**Symptoms:**
- Error message when updating profile
- Changes not saved
- HTTP 400 or 403 response

**Possible Causes and Solutions:**

1. **Validation Errors**
   - Check the error response for specific validation issues
   - Ensure all required fields are provided
   - Verify data formats (email format, phone number format, etc.)

2. **Permission Issues**
   - You may not have permission to update certain fields
   - Contact administrator to review account permissions

### API Key Issues

#### Issue: API Keys Not Working

**Symptoms:**
- Error message: "Invalid API key"
- HTTP 401 Unauthorized response
- Unable to use API

**Possible Causes and Solutions:**

1. **Incorrect API Key**
   - Verify you're using the correct API key
   - Check for leading/trailing spaces

2. **Expired API Key**
   - API keys may have expiration dates
   - Generate a new API key

3. **Revoked API Key**
   - API keys may be revoked for security reasons
   - Generate a new API key

## Debugging Techniques

### Check JWT Token Contents

You can decode (but not verify) JWT tokens at [jwt.io](https://jwt.io/) to inspect their contents:

1. Paste your JWT token in the debugger
2. Check the payload for:
   - `exp` (expiration time)
   - `sub` (subject, usually user ID)
   - Other claims

> **Security Note**: Do not paste production JWT tokens on public websites. Use a local tool if possible.

### Enable Verbose Logging

When running authentication tests with `run_auth_test.sh`, use the verbose flag:

```bash
./run_auth_test.sh --verbose
```

This will provide detailed logging information for troubleshooting.

### Check Authentication Headers

When making API requests, ensure your Authorization header is correctly formatted:

```
Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...
```

Common mistakes include:
- Missing the "Bearer" prefix
- Extra spaces
- URL-encoded tokens (should be plain)

### Review Logs

Check the application logs for more detailed error information:

```bash
# View authentication flow test logs
cat jwt_api_docs/auth_flow_test.log
```

## Advanced Troubleshooting

### JWT Signature Verification Failures

If the server reports that the JWT signature is invalid:

1. Ensure you haven't modified the token
2. Check that the correct signing algorithm is being used
3. Verify that the server has the correct signing keys

### Cross-Origin Issues

If using JWT authentication in a web application:

1. Ensure the server has proper CORS headers configured
2. Check that cookies (if used) are properly configured for cross-origin requests
3. Verify that the Authorization header is included in preflight requests

## Contact Support

If you've tried the solutions above and still experience issues:

1. Run the authentication test with verbose logging:
   ```bash
   ./run_auth_test.sh --verbose
   ```

2. Collect the log file:
   ```
   jwt_api_docs/auth_flow_test.log
   ```

3. Contact Hyperswitch support with:
   - The log file
   - A description of the issue
   - Steps to reproduce
   - Your environment details 