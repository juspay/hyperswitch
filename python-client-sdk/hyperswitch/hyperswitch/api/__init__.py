"""Contains methods for accessing the API"""

class API:
    def __init__(self, client):
        self.client = client
        self.merchant_account = MerchantAccountAPI(client)
        self.api_key = APIKeyAPI(client)
        self.payments = PaymentsAPI(client)

class MerchantAccountAPI:
    def __init__(self, client):
        self.client = client

    def create(self, body, **kwargs):
        from .merchant_account import create_a_merchant_account
        return create_a_merchant_account.sync_detailed(client=self.client, body=body, **kwargs)

    def delete(self, merchant_id, **kwargs):
        from .merchant_account import delete_a_merchant_account
        return delete_a_merchant_account.sync_detailed(client=self.client, merchant_id=merchant_id, **kwargs)

    # TODO: change to account_id
    def retrieve(self, merchant_id, **kwargs):
        from .merchant_account import retrieve_a_merchant_account
        return retrieve_a_merchant_account.sync_detailed(client=self.client, account_id=merchant_id, **kwargs)

    # TODO: change to account_id
    def update(self, merchant_id, body, **kwargs):
        from .merchant_account import update_a_merchant_account
        return update_a_merchant_account.sync_detailed(client=self.client, account_id=merchant_id, body=body, **kwargs)

class APIKeyAPI:
    def __init__(self, client):
        self.client = client

    def create(self, body, **kwargs):
        from .api_key import create_an_api_key
        return create_an_api_key.sync_detailed(client=self.client, body=body, **kwargs)

    def delete(self, api_key_id, **kwargs):
        from .api_key import delete_an_api_key
        return delete_an_api_key.sync_detailed(client=self.client, api_key_id=api_key_id, **kwargs)

    # TODO: change to key_id
    def retrieve(self, api_key_id, **kwargs):
        from .api_key import retrieve_an_api_key
        return retrieve_an_api_key.sync_detailed(client=self.client, key_id=api_key_id, **kwargs)
    
    # TODO: change to key_id
    def update(self, key_id, body, **kwargs):
        from .api_key import update_an_api_key
        return update_an_api_key.sync_detailed(client=self.client, key_id=key_id, body=body, **kwargs)

class PaymentsAPI:
    def __init__(self, client):
        self.client = client

    def create(self, body, **kwargs):
        from .payments import create_a_payment
        return create_a_payment.sync_detailed(client=self.client, body=body, **kwargs)

    def retrieve(self, payment_id, **kwargs):
        from .payments import retrieve_a_payment
        return retrieve_a_payment.sync_detailed(client=self.client, payment_id=payment_id, **kwargs)

    def update(self, payment_id, body, **kwargs):
        from .payments import update_a_payment
        return update_a_payment.sync_detailed(client=self.client, payment_id=payment_id, body=body, **kwargs)

    def cancel(self, payment_id, **kwargs):
        from .payments import cancel_a_payment
        return cancel_a_payment.sync_detailed(client=self.client, payment_id=payment_id, **kwargs)
