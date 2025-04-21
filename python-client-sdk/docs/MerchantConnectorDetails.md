# MerchantConnectorDetails


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**connector_account_details** | **object** | Account details of the Connector. You can specify up to 50 keys, with key names up to 40 characters long and values up to 500 characters long. Useful for storing additional, structured information on an object. | [optional] 
**metadata** | **object** | Metadata is useful for storing additional, unstructured information on an object. | [optional] 

## Example

```python
from hyperswitch.models.merchant_connector_details import MerchantConnectorDetails

# TODO update the JSON string below
json = "{}"
# create an instance of MerchantConnectorDetails from a JSON string
merchant_connector_details_instance = MerchantConnectorDetails.from_json(json)
# print the JSON string representation of the object
print(MerchantConnectorDetails.to_json())

# convert the object into a dict
merchant_connector_details_dict = merchant_connector_details_instance.to_dict()
# create an instance of MerchantConnectorDetails from a dict
merchant_connector_details_from_dict = MerchantConnectorDetails.from_dict(merchant_connector_details_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


