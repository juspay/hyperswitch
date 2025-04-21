import pytest
from tests.api.merchant_connector_account_api import MerchantConnectorAccountApi

@pytest.fixture
def connector_id(admin_client, merchant_id, profile_id):
    """Create a new merchant connector and return its ID."""
    global _connector_id

    # If we already have a connector ID, return it
    if _connector_id is not None:
        logger.info(f"Using existing connector ID: {_connector_id}")
        return _connector_id

    logger.info("Creating new merchant connector for testing")
    connector_api = MerchantConnectorAccountApi(admin_client)

    # Create connector details
    # Use a simple valid structure for testing 