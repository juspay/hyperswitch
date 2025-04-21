# Hyperswitch Python Client SDK Setup and Testing Guide

## Prerequisites

- Python 3.12.3 or higher
- pip (Python package installer)
- virtualenv (for creating isolated Python environments)
- Node.js (for OpenAPI Generator CLI)
- Java (required by OpenAPI Generator)

## Client Generation

The Python client SDK is automatically generated from the OpenAPI specification using the OpenAPI Generator tool. Here's how the process works:

1. **OpenAPI Specification Source**
   - The specification is located at `api-reference/openapi_spec.json`
   - This JSON file contains the complete API definition including:
     - Endpoints
     - Request/Response models
     - Authentication methods
     - Data types and schemas

2. **Generating the Client**
   ```bash
   npx @openapitools/openapi-generator-cli generate \
     -i api-reference/openapi_spec.json \
     -g python \
     -o python-client-sdk \
     --additional-properties=packageName=hyperswitch,projectName=hyperswitch-python-client-sdk,packageVersion=1.0.0
   ```

   This command:
   - Uses the OpenAPI Generator CLI
   - Takes the OpenAPI specification as input
   - Generates a Python client (`-g python`)
   - Outputs to the `python-client-sdk` directory
   - Configures package details

3. **Generated Components**
   The generator creates:
   - API client classes
   - Model classes for all schemas
   - Configuration and authentication handling
   - Request/response serialization
   - Type hints and documentation

4. **Updating the Client**
   When the API specification changes:
   1. Update the OpenAPI specification file
   2. Run the generator command again
   3. Review and test the changes
   4. Update version numbers if needed

## Environment Setup

1. Create and activate a virtual environment:
```bash
python -m venv venv
source venv/bin/activate  # On Linux/Mac
# or
.\venv\Scripts\activate  # On Windows
```

2. Install development dependencies:
```bash
pip install -r requirements-dev.txt
```

## Project Structure

The Python client SDK is organized as follows:

```
python-client-sdk/
├── hyperswitch/
│   └── hyperswitch/
│       ├── api/
│       ├── models/
│       └── client.py
├── tests/
│   └── test_admin.py
├── requirements-dev.txt
└── setup_information.md
```

## Key Components

### Models

The SDK provides several key models for interacting with the Hyperswitch API:

1. `MerchantAccountCreate`: For creating merchant accounts
   - Required fields: `merchant_id`, `merchant_name`
   - Optional fields: `merchant_details`, `return_url`, `webhook_details`, etc.

2. `UpdateApiKeyRequest`: For creating and managing API keys
   - Fields: `name`, `description`, `expiration`

3. `MerchantDetails`: For merchant information
   - Fields: `primary_contact_person`, `primary_email`, `primary_phone`, etc.

4. `AddressDetails`: For merchant address information
   - Fields: `line1`, `line2`, `city`, `state`, `zip_`, `country`

### Authentication

The SDK uses an `AuthenticatedClient` for making authenticated requests:

```python
from hyperswitch.hyperswitch import AuthenticatedClient

client = AuthenticatedClient(
    base_url="http://localhost:8080",
    token="your_api_key",
    prefix="",
    auth_header_name="api-key"
)
```

## Testing

The test suite includes two main test cases:

### 1. Merchant Creation Test

Tests the creation of a merchant account with:
- Address details
- Contact information
- Webhook configuration
- Business details

```python
def test_create_merchant(merchant_id):
    assert merchant_id is not None
    assert merchant_id.startswith("test_merchant_")
```

### 2. API Key Creation Test

Tests the creation of an API key for a merchant:
- Creates a merchant (using fixture)
- Creates an API key
- Verifies the API key response

```python
def test_create_api_key(admin_client, merchant_id):
    api_key_request = UpdateApiKeyRequest(
        name="Test API Key",
        description="API key for testing",
        expiration=ApiKeyExpirationType0.NEVER
    )
    
    response = admin_client.api_key.create(merchant_id=merchant_id, body=api_key_request)
    assert response.status_code == 200
    assert response.parsed.api_key is not None
```

## Running Tests

To run the test suite:

```bash
python -m pytest tests/test_admin.py -v
```

## Environment Variables

The tests use the following environment variable:
- `HYPERSWITCH_ADMIN_API_KEY`: The admin API key for authentication
  - Default value: "test_admin" (for testing purposes)

## Best Practices

1. Always use fixtures for shared setup code
2. Keep test functions focused on assertions
3. Use descriptive test names
4. Include proper error handling
5. Document test dependencies

## Troubleshooting

Common issues and solutions:

1. **Import Errors**
   - Ensure all dependencies are installed
   - Check Python path and virtual environment activation

2. **Authentication Errors**
   - Verify API key is correct
   - Check base URL configuration

3. **Test Failures**
   - Check network connectivity
   - Verify test data is valid
   - Ensure all required fields are provided

## Next Steps

1. Add more test cases for different scenarios
2. Implement error handling tests
3. Add integration tests
4. Document API usage examples 