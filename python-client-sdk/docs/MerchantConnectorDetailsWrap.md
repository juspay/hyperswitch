# MerchantConnectorDetailsWrap

Merchant connector details used to make payments.

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**creds_identifier** | **str** | Creds Identifier is to uniquely identify the credentials. Do not send any sensitive info, like encoded_data in this field. And do not send the string \&quot;null\&quot;. | 
**encoded_data** | [**MerchantConnectorDetails**](MerchantConnectorDetails.md) |  | [optional] 

## Example

```python
from hyperswitch.models.merchant_connector_details_wrap import MerchantConnectorDetailsWrap

# TODO update the JSON string below
json = "{}"
# create an instance of MerchantConnectorDetailsWrap from a JSON string
merchant_connector_details_wrap_instance = MerchantConnectorDetailsWrap.from_json(json)
# print the JSON string representation of the object
print(MerchantConnectorDetailsWrap.to_json())

# convert the object into a dict
merchant_connector_details_wrap_dict = merchant_connector_details_wrap_instance.to_dict()
# create an instance of MerchantConnectorDetailsWrap from a dict
merchant_connector_details_wrap_from_dict = MerchantConnectorDetailsWrap.from_dict(merchant_connector_details_wrap_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


