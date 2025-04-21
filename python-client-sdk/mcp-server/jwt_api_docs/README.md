# Hyperswitch JWT-Based API Documentation

This documentation covers the JWT-based authentication and API system for Hyperswitch, providing comprehensive guides for implementation, testing, and troubleshooting.

## Overview

Hyperswitch's JWT-based authentication system enables secure access to platform resources through JSON Web Tokens. This documentation covers the full lifecycle of authentication, user management, and API access, primarily focusing on the MCP tools and test scripts within this directory.

## Current Status (As of 2025-04-20)

*   **MCP Tools (`test_tools.py`):**
    *   **Working:** Core Authentication (Sign-in, Terminate 2FA, Get User Info), Change Password.
    *   **Blocked (Backend API Issues):** Password Reset (API expects incorrect payload), Email Verification (API rejects valid JWT with 401). See [Troubleshooting Guide](troubleshooting.md) for details.
    *   **Untested:** Sign Out.
*   **Core Python SDK (`pytest` in parent dir):**
    *   **Pending Investigation:** Multiple test failures observed related to Merchant Connector and Profile APIs. Requires debugging.

## Documentation Structure

### Authentication Flow

- [Authentication Flow Test Script](auth_flow_test.py) - Python script to test the complete authentication flow
- [Test Runner Script](run_auth_test.sh) - Shell script to run the authentication tests with various options
- [Troubleshooting Guide](troubleshooting.md) - Solutions for common authentication issues and known problems.

### Testing

- [MCP Tools Test Script](test_tools.py) - Python script to test the Hyperswitch MCP tools (see status above).
- [Test Report](test_report.md) - Comprehensive report of JWT authentication testing results (may need update).
- [Group 1 Test Results](group1_authentication/test_results.md) - Test results for User Authentication APIs (may need update).

### API Groups

#### Group 1: User Authentication

- [Implementation Plan](group1_user_auth/plan.md) - Detailed plan for the User Authentication API group
- [Execution Document](group1_user_auth/execution.md) - Implementation status and progress

#### Group 2: User Management

- [Implementation Plan](group2_user_management/plan.md) - Detailed plan for the User Management API group
- [Execution Document](group2_user_management/execution.md) - Implementation status and progress

#### Group 3: API Keys Management

- [Implementation Plan](group3_api_keys/plan.md) - Detailed plan for the API Keys Management group
- [Execution Document](group3_api_keys/execution.md) - Implementation status and progress
- [Troubleshooting Guide](group3_api_keys/troubleshooting.md) - API Keys specific troubleshooting

#### Group 4: Business Profiles

- [Implementation Plan](group4_business_profiles/plan.md) - Detailed plan for the Business Profiles API group
- [Execution Document](group4_business_profiles/execution.md) - Implementation status and progress

## Getting Started

To get started with testing the authentication flow using the alternative script:

1.  Ensure you have Python 3 installed
2.  Run the test script:

```bash
chmod +x run_auth_test.sh
./run_auth_test.sh --help
```

For testing the MCP tools directly using `test_tools.py`:

```bash
# Run all available tests (Note: reset_pwd and verify_email will fail due to known issues)
python test_tools.py --email jarnura47@gmail.com --password <PASSWORD> --test all

# Run only the core auth flow
python test_tools.py --email jarnura47@gmail.com --password <PASSWORD> --test auth

# Run password change test (will prompt for current password if not provided)
# Known working password for jarnura47@gmail.com: NewPa$$w0rd123_1745154955
python test_tools.py --email jarnura47@gmail.com --password <CURRENT_PASSWORD> --new-password <NEW_PASSWORD> --test change_pwd

# Run password reset initiation (EXPECTED TO FAIL - Backend Issue)
# Will prompt for reset token from email (if initiation were successful) and new password
python test_tools.py --email jarnura47@gmail.com --new-password <NEW_PASSWORD> --test reset_pwd
```

Replace `<PASSWORD>`, `<CURRENT_PASSWORD>`, `<NEW_PASSWORD>` as needed. If `--password` is omitted for flows requiring it, you will be prompted.

For details on specific API implementations, refer to the implementation plans and execution documents in each group's directory.

## Security Considerations

The JWT-based authentication system implements several security best practices:

- Short-lived JWT tokens (1 hour expiration)
- Secure token storage recommendations
- Two-factor authentication support
- API key rotation capabilities
- Rate limiting on authentication endpoints

For more details on security, refer to the implementation plans for each API group.

## Contributing

When contributing to the JWT API documentation:

1. Follow the established documentation structure
2. Update execution documents as APIs are implemented
3. Add test cases to the authentication flow test script as needed
4. Document any security considerations or best practices

## Roadmap

Current development priorities:

1. Complete implementation of User Authentication API group
2. Implement User Management API group
3. Develop API Keys Management functionality
4. Add Business Profiles API group

See each group's execution document for detailed implementation status and plans. 