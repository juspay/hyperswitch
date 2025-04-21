# MerchantAccountDeleteResponse


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**merchant_id** | **str** | The identifier for the Merchant Account | 
**deleted** | **bool** | If the connector is deleted or not | 

## Example

```python
from hyperswitch.models.merchant_account_delete_response import MerchantAccountDeleteResponse

# TODO update the JSON string below
json = "{}"
# create an instance of MerchantAccountDeleteResponse from a JSON string
merchant_account_delete_response_instance = MerchantAccountDeleteResponse.from_json(json)
# print the JSON string representation of the object
print(MerchantAccountDeleteResponse.to_json())

# convert the object into a dict
merchant_account_delete_response_dict = merchant_account_delete_response_instance.to_dict()
# create an instance of MerchantAccountDeleteResponse from a dict
merchant_account_delete_response_from_dict = MerchantAccountDeleteResponse.from_dict(merchant_account_delete_response_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


