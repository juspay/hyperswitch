# MerchantConnectorDeleteResponse


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**merchant_id** | **str** | The identifier for the Merchant Account | 
**merchant_connector_id** | **str** | Unique ID of the connector | 
**deleted** | **bool** | If the connector is deleted or not | 

## Example

```python
from hyperswitch.models.merchant_connector_delete_response import MerchantConnectorDeleteResponse

# TODO update the JSON string below
json = "{}"
# create an instance of MerchantConnectorDeleteResponse from a JSON string
merchant_connector_delete_response_instance = MerchantConnectorDeleteResponse.from_json(json)
# print the JSON string representation of the object
print(MerchantConnectorDeleteResponse.to_json())

# convert the object into a dict
merchant_connector_delete_response_dict = merchant_connector_delete_response_instance.to_dict()
# create an instance of MerchantConnectorDeleteResponse from a dict
merchant_connector_delete_response_from_dict = MerchantConnectorDeleteResponse.from_dict(merchant_connector_delete_response_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


