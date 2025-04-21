# MerchantAccountCreate


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**merchant_id** | **str** | The identifier for the Merchant Account | 
**merchant_name** | **str** | Name of the Merchant Account | [optional] 
**merchant_details** | [**MerchantDetails**](MerchantDetails.md) |  | [optional] 
**return_url** | **str** | The URL to redirect after the completion of the operation | [optional] 
**webhook_details** | [**WebhookDetails**](WebhookDetails.md) |  | [optional] 
**payout_routing_algorithm** | [**RoutingAlgorithm**](RoutingAlgorithm.md) |  | [optional] 
**sub_merchants_enabled** | **bool** | A boolean value to indicate if the merchant is a sub-merchant under a master or a parent merchant. By default, its value is false. | [optional] [default to False]
**parent_merchant_id** | **str** | Refers to the Parent Merchant ID if the merchant being created is a sub-merchant | [optional] 
**enable_payment_response_hash** | **bool** | A boolean value to indicate if payment response hash needs to be enabled | [optional] [default to False]
**payment_response_hash_key** | **str** | Refers to the hash key used for calculating the signature for webhooks and redirect response. If the value is not provided, a value is automatically generated. | [optional] 
**redirect_to_merchant_with_http_post** | **bool** | A boolean value to indicate if redirect to merchant with http post needs to be enabled. | [optional] [default to False]
**metadata** | **object** | Metadata is useful for storing additional, unstructured information on an object | [optional] 
**publishable_key** | **str** | API key that will be used for client side API access. A publishable key has to be always paired with a &#x60;client_secret&#x60;. A &#x60;client_secret&#x60; can be obtained by creating a payment with &#x60;confirm&#x60; set to false | [optional] 
**locker_id** | **str** | An identifier for the vault used to store payment method information. | [optional] 
**primary_business_details** | [**PrimaryBusinessDetails**](PrimaryBusinessDetails.md) |  | [optional] 
**frm_routing_algorithm** | **object** | The frm routing algorithm to be used for routing payments to desired FRM&#39;s | [optional] 
**organization_id** | **str** | The id of the organization to which the merchant belongs to, if not passed an organization is created | [optional] 
**pm_collect_link_config** | [**BusinessCollectLinkConfig**](BusinessCollectLinkConfig.md) |  | [optional] 
**product_type** | [**MerchantProductType**](MerchantProductType.md) |  | [optional] 

## Example

```python
from hyperswitch.models.merchant_account_create import MerchantAccountCreate

# TODO update the JSON string below
json = "{}"
# create an instance of MerchantAccountCreate from a JSON string
merchant_account_create_instance = MerchantAccountCreate.from_json(json)
# print the JSON string representation of the object
print(MerchantAccountCreate.to_json())

# convert the object into a dict
merchant_account_create_dict = merchant_account_create_instance.to_dict()
# create an instance of MerchantAccountCreate from a dict
merchant_account_create_from_dict = MerchantAccountCreate.from_dict(merchant_account_create_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


