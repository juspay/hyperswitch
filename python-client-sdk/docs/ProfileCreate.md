# ProfileCreate


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**profile_name** | **str** | The name of profile | [optional] 
**return_url** | **str** | The URL to redirect after the completion of the operation | [optional] 
**enable_payment_response_hash** | **bool** | A boolean value to indicate if payment response hash needs to be enabled | [optional] [default to True]
**payment_response_hash_key** | **str** | Refers to the hash key used for calculating the signature for webhooks and redirect response. If the value is not provided, a value is automatically generated. | [optional] 
**redirect_to_merchant_with_http_post** | **bool** | A boolean value to indicate if redirect to merchant with http post needs to be enabled | [optional] [default to False]
**webhook_details** | [**WebhookDetails**](WebhookDetails.md) |  | [optional] 
**metadata** | **object** | Metadata is useful for storing additional, unstructured information on an object. | [optional] 
**routing_algorithm** | **object** | The routing algorithm to be used for routing payments to desired connectors | [optional] 
**intent_fulfillment_time** | **int** | Will be used to determine the time till which your payment will be active once the payment session starts | [optional] 
**frm_routing_algorithm** | **object** | The frm routing algorithm to be used for routing payments to desired FRM&#39;s | [optional] 
**payout_routing_algorithm** | [**RoutingAlgorithm**](RoutingAlgorithm.md) |  | [optional] 
**applepay_verified_domains** | **List[str]** | Verified Apple Pay domains for a particular profile | [optional] 
**session_expiry** | **int** | Client Secret Default expiry for all payments created under this profile | [optional] 
**payment_link_config** | [**BusinessPaymentLinkConfig**](BusinessPaymentLinkConfig.md) |  | [optional] 
**authentication_connector_details** | [**AuthenticationConnectorDetails**](AuthenticationConnectorDetails.md) |  | [optional] 
**use_billing_as_payment_method_billing** | **bool** | Whether to use the billing details passed when creating the intent as payment method billing | [optional] 
**collect_shipping_details_from_wallet_connector** | **bool** | A boolean value to indicate if customer shipping details needs to be collected from wallet connector only if it is required field for connector (Eg. Apple Pay, Google Pay etc) | [optional] [default to False]
**collect_billing_details_from_wallet_connector** | **bool** | A boolean value to indicate if customer billing details needs to be collected from wallet connector only if it is required field for connector (Eg. Apple Pay, Google Pay etc) | [optional] [default to False]
**always_collect_shipping_details_from_wallet_connector** | **bool** | A boolean value to indicate if customer shipping details needs to be collected from wallet connector irrespective of connector required fields (Eg. Apple pay, Google pay etc) | [optional] [default to False]
**always_collect_billing_details_from_wallet_connector** | **bool** | A boolean value to indicate if customer billing details needs to be collected from wallet connector irrespective of connector required fields (Eg. Apple pay, Google pay etc) | [optional] [default to False]
**is_connector_agnostic_mit_enabled** | **bool** | Indicates if the MIT (merchant initiated transaction) payments can be made connector agnostic, i.e., MITs may be processed through different connector than CIT (customer initiated transaction) based on the routing rules. If set to &#x60;false&#x60;, MIT will go through the same connector as the CIT. | [optional] 
**payout_link_config** | [**BusinessPayoutLinkConfig**](BusinessPayoutLinkConfig.md) |  | [optional] 
**outgoing_webhook_custom_http_headers** | **object** | These key-value pairs are sent as additional custom headers in the outgoing webhook request. It is recommended not to use more than four key-value pairs. | [optional] 
**tax_connector_id** | **str** | Merchant Connector id to be stored for tax_calculator connector | [optional] 
**is_tax_connector_enabled** | **bool** | Indicates if tax_calculator connector is enabled or not. If set to &#x60;true&#x60; tax_connector_id will be checked. | [optional] 
**is_network_tokenization_enabled** | **bool** | Indicates if network tokenization is enabled or not. | [optional] 
**is_auto_retries_enabled** | **bool** | Indicates if is_auto_retries_enabled is enabled or not. | [optional] 
**max_auto_retries_enabled** | **int** | Maximum number of auto retries allowed for a payment | [optional] 
**always_request_extended_authorization** | **bool** | Bool indicating if extended authentication must be requested for all payments | [optional] 
**is_click_to_pay_enabled** | **bool** | Indicates if click to pay is enabled or not. | [optional] 
**authentication_product_ids** | **object** | Product authentication ids | [optional] 
**card_testing_guard_config** | [**CardTestingGuardConfig**](CardTestingGuardConfig.md) |  | [optional] 
**is_clear_pan_retries_enabled** | **bool** | Indicates if clear pan retries is enabled or not. | [optional] 
**force_3ds_challenge** | **bool** | Indicates if 3ds challenge is forced | [optional] 
**is_debit_routing_enabled** | **bool** | Indicates if debit routing is enabled or not | [optional] 
**merchant_business_country** | [**CountryAlpha2**](CountryAlpha2.md) |  | [optional] 

## Example

```python
from hyperswitch.models.profile_create import ProfileCreate

# TODO update the JSON string below
json = "{}"
# create an instance of ProfileCreate from a JSON string
profile_create_instance = ProfileCreate.from_json(json)
# print the JSON string representation of the object
print(ProfileCreate.to_json())

# convert the object into a dict
profile_create_dict = profile_create_instance.to_dict()
# create an instance of ProfileCreate from a dict
profile_create_from_dict = ProfileCreate.from_dict(profile_create_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


