# MerchantAccountResponse


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**merchant_id** | **str** | The identifier for the Merchant Account | 
**merchant_name** | **str** | Name of the Merchant Account | [optional] 
**return_url** | **str** | The URL to redirect after completion of the payment | [optional] 
**enable_payment_response_hash** | **bool** | A boolean value to indicate if payment response hash needs to be enabled | [default to False]
**payment_response_hash_key** | **str** | Refers to the hash key used for calculating the signature for webhooks and redirect response. If the value is not provided, a value is automatically generated. | [optional] 
**redirect_to_merchant_with_http_post** | **bool** | A boolean value to indicate if redirect to merchant with http post needs to be enabled | [default to False]
**merchant_details** | [**MerchantDetails**](MerchantDetails.md) |  | [optional] 
**webhook_details** | [**WebhookDetails**](WebhookDetails.md) |  | [optional] 
**payout_routing_algorithm** | [**RoutingAlgorithm**](RoutingAlgorithm.md) |  | [optional] 
**sub_merchants_enabled** | **bool** | A boolean value to indicate if the merchant is a sub-merchant under a master or a parent merchant. By default, its value is false. | [optional] [default to False]
**parent_merchant_id** | **str** | Refers to the Parent Merchant ID if the merchant being created is a sub-merchant | [optional] 
**publishable_key** | **str** | API key that will be used for server side API access | [optional] 
**metadata** | **object** | Metadata is useful for storing additional, unstructured information on an object. | [optional] 
**locker_id** | **str** | An identifier for the vault used to store payment method information. | [optional] 
**primary_business_details** | [**List[PrimaryBusinessDetails]**](PrimaryBusinessDetails.md) | Details about the primary business unit of the merchant account | 
**frm_routing_algorithm** | [**RoutingAlgorithm**](RoutingAlgorithm.md) |  | [optional] 
**organization_id** | **str** | The organization id merchant is associated with | 
**is_recon_enabled** | **bool** | A boolean value to indicate if the merchant has recon service is enabled or not, by default value is false | 
**default_profile** | **str** | The default profile that must be used for creating merchant accounts and payments | [optional] 
**recon_status** | [**ReconStatus**](ReconStatus.md) |  | 
**pm_collect_link_config** | [**BusinessCollectLinkConfig**](BusinessCollectLinkConfig.md) |  | [optional] 
**product_type** | [**MerchantProductType**](MerchantProductType.md) |  | [optional] 

## Example

```python
from hyperswitch.models.merchant_account_response import MerchantAccountResponse

# TODO update the JSON string below
json = "{}"
# create an instance of MerchantAccountResponse from a JSON string
merchant_account_response_instance = MerchantAccountResponse.from_json(json)
# print the JSON string representation of the object
print(MerchantAccountResponse.to_json())

# convert the object into a dict
merchant_account_response_dict = merchant_account_response_instance.to_dict()
# create an instance of MerchantAccountResponse from a dict
merchant_account_response_from_dict = MerchantAccountResponse.from_dict(merchant_account_response_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


