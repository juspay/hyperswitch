# MerchantConnectorId


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**merchant_id** | **str** |  | 
**merchant_connector_id** | **str** |  | 

## Example

```python
from hyperswitch.models.merchant_connector_id import MerchantConnectorId

# TODO update the JSON string below
json = "{}"
# create an instance of MerchantConnectorId from a JSON string
merchant_connector_id_instance = MerchantConnectorId.from_json(json)
# print the JSON string representation of the object
print(MerchantConnectorId.to_json())

# convert the object into a dict
merchant_connector_id_dict = merchant_connector_id_instance.to_dict()
# create an instance of MerchantConnectorId from a dict
merchant_connector_id_from_dict = MerchantConnectorId.from_dict(merchant_connector_id_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


