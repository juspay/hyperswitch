# API Keys Management Usage Guide

This guide provides comprehensive instructions for managing API keys in the Hyperswitch platform, covering the creation, usage, and management of API keys.

## Overview

API keys in Hyperswitch provide secure programmatic access to our API endpoints. They are essential for integrating Hyperswitch services into your applications and systems.

## Types of API Keys

Hyperswitch supports two types of API keys:

### Secret Keys

- **Format**: `sk_test_*` (test) or `sk_live_*` (production)
- **Usage**: Server-side operations, including sensitive operations
- **Security**: Must be kept secure and never exposed to clients
- **Capabilities**: Full access to authorized API endpoints

### Publishable Keys

- **Format**: `pk_test_*` (test) or `pk_live_*` (production)
- **Usage**: Client-side operations in web or mobile apps
- **Security**: Can be included in client-side code
- **Capabilities**: Limited to non-sensitive operations

## API Key Management Operations

### Listing API Keys

To view all available API keys:

```python
from hyperswitch_mcp.api_keys import list_api_keys

# List all API keys
api_keys = list_api_keys()
print(f"Found {len(api_keys)} API keys")

# Print key details (without revealing full keys)
for key in api_keys:
    print(f"ID: {key.id}")
    print(f"Name: {key.name}")
    print(f"Type: {key.key_type}")
    print(f"Created: {key.created_at}")
    print(f"Expires: {key.expires_at or 'Never'}")
    print("---")
```

### Creating a New API Key

To create a new API key:

```python
from hyperswitch_mcp.api_keys import create_api_key
from hyperswitch_mcp.models.api_key import ApiKeyType, ExpirationPeriod

# Create a new secret key for backend services
new_key = create_api_key(
    name="Backend Service Key",
    description="For server-side API calls",
    key_type=ApiKeyType.SECRET,
    expiration=ExpirationPeriod.NINETY_DAYS
)

# IMPORTANT: This is the only time the full key will be displayed
print(f"Your new API key: {new_key.full_key}")
print(f"Key ID: {new_key.id}")
print(f"Make sure to save this key securely")
```

⚠️ **Important**: The full API key is only displayed once at creation time. Store it securely as it cannot be retrieved later.

### Retrieving API Key Details

To get details of a specific API key:

```python
from hyperswitch_mcp.api_keys import get_api_key

# Get details of a specific key
key_id = "api_key_123456789"
key_details = get_api_key(key_id)

if key_details:
    print(f"Key Name: {key_details.name}")
    print(f"Description: {key_details.description}")
    print(f"Status: {'Active' if key_details.active else 'Inactive'}")
    print(f"Last used: {key_details.last_used_at or 'Never used'}")
else:
    print(f"Key with ID {key_id} not found")
```

### Updating an API Key

To update an existing API key's properties:

```python
from hyperswitch_mcp.api_keys import update_api_key

# Update key properties
key_id = "api_key_123456789"
updated_key = update_api_key(
    key_id=key_id,
    name="Updated Key Name",
    description="Updated description for this key",
    active=True  # Enable or disable the key
)

if updated_key:
    print(f"Key updated successfully: {updated_key.name}")
else:
    print(f"Failed to update key {key_id}")
```

### Deleting an API Key

To delete an API key:

```python
from hyperswitch_mcp.api_keys import delete_api_key

# Delete a key that's no longer needed
key_id = "api_key_123456789"
success = delete_api_key(key_id)

if success:
    print(f"Key {key_id} has been deleted")
else:
    print(f"Failed to delete key {key_id}")
```

## Best Practices for API Key Usage

### Security Considerations

1. **Store keys securely**:
   - Use environment variables or secure vaults
   - Never hardcode keys in your application
   - Don't store keys in version control systems

2. **Use the right key type**:
   - Secret keys for server-side operations only
   - Publishable keys for client-side code

3. **Implement key rotation**:
   - Rotate keys regularly (e.g., every 90 days)
   - Update all systems during rotation periods
   - Maintain an inventory of where keys are used

4. **Monitor key usage**:
   - Track and audit key usage
   - Watch for unusual activity patterns
   - Revoke compromised keys immediately

### Implementation Examples

#### Server-side Implementation (Node.js)

```javascript
// Environment setup with secure key storage
require('dotenv').config();
const hyperswitchKey = process.env.HYPERSWITCH_SECRET_KEY;

// Making API calls with the key
const makePaymentRequest = async (paymentData) => {
  const response = await fetch('https://api.hyperswitch.io/payments', {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      'Authorization': `Bearer ${hyperswitchKey}`
    },
    body: JSON.stringify(paymentData)
  });
  return response.json();
};
```

#### Client-side Implementation (JavaScript)

```javascript
// Safe to use publishable key in client code
const hyperswitchPublishableKey = 'pk_test_abc123...';

// Initialize Hyperswitch client-side library
const hyperswitch = new HyperSwitchClient({
  publishableKey: hyperswitchPublishableKey,
  // other configuration options
});

// Use for allowed client-side operations
hyperswitch.createPaymentMethod({
  type: 'card',
  card: {
    number: '4242424242424242',
    exp_month: 12,
    exp_year: 2025,
    cvc: '123'
  }
}).then(paymentMethod => {
  // Handle the payment method
}).catch(error => {
  // Handle errors
});
```

## API Key Expiration Management

### Setting Expiration Periods

When creating API keys, you can select from several expiration options:

- `ExpirationPeriod.NEVER`: Key never expires (use with caution)
- `ExpirationPeriod.THIRTY_DAYS`: Expires after 30 days
- `ExpirationPeriod.NINETY_DAYS`: Expires after 90 days
- `ExpirationPeriod.ONE_YEAR`: Expires after 1 year
- `ExpirationPeriod.CUSTOM`: Set a custom expiration date

### Handling Expiring Keys

1. **Monitor expiration dates**:
   ```python
   from hyperswitch_mcp.api_keys import list_api_keys
   from datetime import datetime, timedelta
   
   # Get keys expiring in the next 7 days
   now = datetime.now()
   soon = now + timedelta(days=7)
   
   api_keys = list_api_keys()
   expiring_keys = [key for key in api_keys 
                    if key.expires_at and now < key.expires_at <= soon]
   
   for key in expiring_keys:
       print(f"Key '{key.name}' expires on {key.expires_at}")
   ```

2. **Key rotation procedure**:
   - Create a new key before the old one expires
   - Update systems to use the new key
   - Verify all systems are working with the new key
   - Delete the old key after its expiration date

## Error Handling

When working with API keys, implement proper error handling:

```python
from hyperswitch_mcp.api_keys import create_api_key
from hyperswitch_mcp.exceptions import ApiKeyError

try:
    new_key = create_api_key(
        name="Production Payment Processing",
        key_type="secret"
    )
    # Handle successful key creation
except ApiKeyError as e:
    print(f"Error creating API key: {e}")
    # Implement appropriate error handling
    if "maximum number" in str(e):
        # Handle too many keys error
        print("Consider deleting unused keys")
    elif "permission" in str(e):
        # Handle permission error
        print("Insufficient permissions to create keys")
    else:
        # Handle other errors
        print("Please check your request and try again")
```

## Working with Business Profiles

If you have multiple business profiles, you can specify which profile to use:

```python
from hyperswitch_mcp.api_keys import list_api_keys

# List keys for a specific business profile
profile_id = "bp_12345"
api_keys = list_api_keys(business_profile_id=profile_id)

print(f"Found {len(api_keys)} API keys for business profile {profile_id}")
```

## Programmatic Examples

### Complete Key Management Workflow

```python
from hyperswitch_mcp.api_keys import create_api_key, get_api_key, update_api_key, delete_api_key
from hyperswitch_mcp.models.api_key import ApiKeyType, ExpirationPeriod
import time

# 1. Create a new key
new_key = create_api_key(
    name="Integration Test Key",
    description="Temporary key for testing",
    key_type=ApiKeyType.SECRET,
    expiration=ExpirationPeriod.THIRTY_DAYS
)

# Store the full key securely
full_key = new_key.full_key
key_id = new_key.id
print(f"Created key: {key_id}")

# 2. Use the key for operations
# ... application code using the key ...

# 3. Retrieve key details
time.sleep(1)  # Wait a moment for system processing
key_details = get_api_key(key_id)
print(f"Key status: {'Active' if key_details.active else 'Inactive'}")

# 4. Update key properties
updated_key = update_api_key(
    key_id=key_id,
    name="Updated Test Key",
    description="Key renamed after testing"
)
print(f"Updated key name: {updated_key.name}")

# 5. Delete the key when no longer needed
success = delete_api_key(key_id)
print(f"Key deleted: {success}")
```

### Key Rotation Example

```python
from hyperswitch_mcp.api_keys import create_api_key, list_api_keys, delete_api_key
from hyperswitch_mcp.models.api_key import ApiKeyType, ExpirationPeriod
import os

def rotate_api_key(key_name, key_description, key_type=ApiKeyType.SECRET):
    """
    Rotate an API key by creating a new one and returning it.
    The caller is responsible for updating systems and deleting the old key.
    """
    # Find existing keys with this name
    all_keys = list_api_keys()
    existing_keys = [k for k in all_keys if k.name == key_name]
    
    # Create new key
    new_key = create_api_key(
        name=key_name,
        description=f"{key_description} (Rotated)",
        key_type=key_type,
        expiration=ExpirationPeriod.NINETY_DAYS
    )
    
    print(f"Created new key: {new_key.id}")
    print(f"IMPORTANT: New key value: {new_key.full_key}")
    
    # Return information about the rotation
    return {
        "new_key_id": new_key.id,
        "new_key": new_key.full_key,
        "old_keys": [k.id for k in existing_keys]
    }

# Example usage:
try:
    rotation_info = rotate_api_key(
        "Production API Key",
        "Main key for production services"
    )
    
    # Store new key securely (e.g., in environment variable)
    os.environ["HYPERSWITCH_API_KEY"] = rotation_info["new_key"]
    
    # Update all services using the key
    # ... code to update services ...
    
    # After confirming all systems use the new key, delete old keys
    for old_key_id in rotation_info["old_keys"]:
        print(f"Ready to delete old key: {old_key_id}")
        # Uncomment when ready to delete:
        # delete_api_key(old_key_id)
        
except Exception as e:
    print(f"Key rotation failed: {e}")
```

## Troubleshooting

For troubleshooting API key issues, refer to the [API Keys Troubleshooting Guide](troubleshooting.md).

## Additional Resources

- [Hyperswitch API Documentation](https://docs.hyperswitch.io/api)
- [Security Best Practices](https://docs.hyperswitch.io/security)
- [API Key Limits and Quotas](https://docs.hyperswitch.io/api-keys/limits)

## Getting Help

For additional assistance with API key management:

- Email: support@hyperswitch.io
- Support portal: https://support.hyperswitch.io
- Developer forum: https://community.hyperswitch.io 