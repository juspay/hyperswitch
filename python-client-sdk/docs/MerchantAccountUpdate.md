# MerchantAccountUpdate


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
**payment_response_hash_key** | **str** | Refers to the hash key used for calculating the signature for webhooks and redirect response. | [optional] 
**redirect_to_merchant_with_http_post** | **bool** | A boolean value to indicate if redirect to merchant with http post needs to be enabled | [optional] [default to False]
**metadata** | **object** | Metadata is useful for storing additional, unstructured information on an object. | [optional] 
**publishable_key** | **str** | API key that will be used for server side API access | [optional] 
**locker_id** | **str** | An identifier for the vault used to store payment method information. | [optional] 
**primary_business_details** | [**List[PrimaryBusinessDetails]**](PrimaryBusinessDetails.md) | Details about the primary business unit of the merchant account | [optional] 
**frm_routing_algorithm** | **object** | The frm routing algorithm to be used for routing payments to desired FRM&#39;s | [optional] 
**default_profile** | **str** | The default profile that must be used for creating merchant accounts and payments | [optional] 
**pm_collect_link_config** | [**BusinessCollectLinkConfig**](BusinessCollectLinkConfig.md) |  | [optional] 

## Example

```python
from hyperswitch.models.merchant_account_update import MerchantAccountUpdate

# TODO update the JSON string below
json = "{}"
# create an instance of MerchantAccountUpdate from a JSON string
merchant_account_update_instance = MerchantAccountUpdate.from_json(json)
# print the JSON string representation of the object
print(MerchantAccountUpdate.to_json())

# convert the object into a dict
merchant_account_update_dict = merchant_account_update_instance.to_dict()
# create an instance of MerchantAccountUpdate from a dict
merchant_account_update_from_dict = MerchantAccountUpdate.from_dict(merchant_account_update_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


