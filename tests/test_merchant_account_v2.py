def test_retrieve_merchant_account(self, admin_client, merchant_id):
    merchant_api = MerchantAccountApi(admin_client) # Use admin_client
    merchant = merchant_api.retrieve_a_merchant_account(merchant_id)
    logger.info(f"Retrieved merchant data: name={merchant.merchant_name}")

def test_update_merchant_account(self, admin_client, merchant_id):
    merchant_api = MerchantAccountApi(admin_client) # Use admin_client

    # First get current merchant data
    current_merchant = merchant_api.retrieve_a_merchant_account(merchant_id) 