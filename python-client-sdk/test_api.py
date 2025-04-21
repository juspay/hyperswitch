from hyperswitch import ApiClient, Configuration
from hyperswitch.api.payments_api import PaymentsApi
from hyperswitch.api.default_api import DefaultApi
from hyperswitch.models.payments_create_request import PaymentsCreateRequest

def test_payment_flow():
    # Configure the client with admin API key
    configuration = Configuration(
        host="http://localhost:8080",  # Local development server
        api_key={"Authorization": "test_admin"}  # Admin API key from development.toml
    )
    api_client = ApiClient(configuration)

    # Initialize APIs
    payments_api = PaymentsApi(api_client)
    default_api = DefaultApi(api_client)

    try:
        # Create a payment
        print("\nCreating payment...")
        payment_request = PaymentsCreateRequest(
            amount=1000,  # Amount in cents
            currency="USD",
            confirm=True,  # Auto-confirm the payment
            payment_method="card",
            payment_method_data={
                "card": {
                    "card_number": "4242424242424242",
                    "card_exp_month": "10",
                    "card_exp_year": "25",
                    "card_holder_name": "John Doe",
                    "card_cvc": "123"
                }
            },
            billing={
                "address": {
                    "line1": "1467",
                    "line2": "Harrison Street",
                    "city": "San Francisco",
                    "state": "California",
                    "zip": "94122",
                    "country": "US"
                }
            }
        )

        payment_response = payments_api.payments_create(payment_request)
        print("Payment Created:", payment_response)

        # Get payment session tokens
        print("\nGetting session tokens...")
        session_tokens = default_api.payments_payment_id_post_session_tokens_post(
            payment_id=payment_response.payment_id
        )
        print("Session Tokens:", session_tokens)

        # Update payment metadata
        print("\nUpdating payment metadata...")
        metadata_response = default_api.payments_payment_id_update_metadata_post(
            payment_id=payment_response.payment_id
        )
        print("Metadata Updated:", metadata_response)

    except Exception as e:
        print(f"Error occurred: {str(e)}")

if __name__ == "__main__":
    test_payment_flow() 