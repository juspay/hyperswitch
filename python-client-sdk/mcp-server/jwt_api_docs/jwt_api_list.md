# Hyperswitch JWT-Based APIs

This document provides a comprehensive list of all Hyperswitch APIs that use JWT-based authentication. The APIs are organized by functional groups.

## Authentication Flow

Before using JWT-authenticated endpoints, you need to go through the following authentication flow:

1. **Sign In** - Authenticate with email/password and receive a TOTP token
2. **Terminate 2FA** - Exchange the TOTP token for a user info JWT token
3. **Use JWT Token** - Include the JWT token in subsequent requests

## API Groups

Based on our analysis, Hyperswitch JWT-based APIs are categorized into the following groups:

### Group 1: User Authentication & Management

**Authentication Flow**
- `/user/signin` - Sign in with email/password, returns TOTP token
- `/user/2fa/terminate` - Verify 2FA and get user JWT token
- `/user/signout` - Sign out and invalidate token

**User Management**
- `/user/info` - Get current user information
- `/user/update` - Update user profile
- `/user/change_password` - Change user password
- `/user/reset_password` - Reset user password (with token)
- `/user/verify_email` - Verify user email (with token)

### Group 2: Business Profiles

**Profile Management**
- `/account/{account_id}/business_profile` - List business profiles
- `/account/{account_id}/business_profile/{profile_id}` - Get specific profile
- `/account/{account_id}/business_profile` - Create business profile
- `/account/{account_id}/business_profile/{profile_id}` - Update business profile
- `/account/{account_id}/business_profile/{profile_id}` - Delete business profile

### Group 3: API Keys Management

**API Keys**
- `/api_keys` - List API keys
- `/api_keys/{key_id}` - Get API key
- `/api_keys` - Create API key
- `/api_keys/{key_id}` - Update API key
- `/api_keys/{key_id}` - Delete API key

### Group 4: Merchant Management

**Merchant Accounts**
- `/merchants` - List merchant accounts
- `/merchants/{merchant_id}` - Get merchant account
- `/merchants` - Create merchant account
- `/merchants/{merchant_id}` - Update merchant account
- `/merchants/{merchant_id}` - Delete merchant account

### Group 5: Roles & Permissions

**User Roles**
- `/user/roles` - List roles
- `/user/roles/{role_id}` - Get role details
- `/user/roles` - Create role
- `/user/roles/{role_id}` - Update role
- `/user/roles/{role_id}` - Delete role

**Invitations**
- `/user/invite` - Create invitation
- `/user/invite/accept` - Accept invitation
- `/user/invitations` - List invitations

### Group 6: Payments & Refunds Reporting

**Payment Reporting**
- `/payments/list` - List payments
- `/payments/filter` - Filter payments
- `/payments/aggregate` - Aggregated payment data

**Refund Reporting**
- `/refunds/list` - List refunds
- `/refunds/filter` - Filter refunds
- `/refunds/aggregate` - Aggregated refund data

### Group 7: Disputes Management

**Disputes**
- `/disputes` - List disputes
- `/disputes/{dispute_id}` - Get dispute
- `/disputes/{dispute_id}` - Respond to dispute
- `/disputes/filter` - Filter disputes

## Implementation Status

Currently, the following APIs are implemented as tools in the MCP SDK:
- `/user/signin` (Sign in to Hyperswitch)
- `/user/2fa/terminate` (Terminate 2FA)
- `/account/{account_id}/business_profile` (List Business Profiles)

The remaining APIs need to be implemented following a similar pattern. 