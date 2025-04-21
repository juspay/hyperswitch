import pytest

@pytest.mark.order(2)
def test_retrieve_api_key(admin_client, merchant_id):
    \"\"\"Test retrieving an API key.\"\"\"
    global _test_api_key_id  # Use the global variable to access the key ID

@pytest.mark.order(3)
def test_update_api_key(admin_client, merchant_id):
    \"\"\"Test updating an API key.\"\"\"
    global _test_api_key_id 