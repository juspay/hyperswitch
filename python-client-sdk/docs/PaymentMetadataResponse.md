# PaymentMetadataResponse


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**payment_id** | **str** | Payment identifier | [optional] 
**metadata** | **object** | Updated metadata | [optional] 

## Example

```python
from hyperswitch.models.payment_metadata_response import PaymentMetadataResponse

# TODO update the JSON string below
json = "{}"
# create an instance of PaymentMetadataResponse from a JSON string
payment_metadata_response_instance = PaymentMetadataResponse.from_json(json)
# print the JSON string representation of the object
print(PaymentMetadataResponse.to_json())

# convert the object into a dict
payment_metadata_response_dict = payment_metadata_response_instance.to_dict()
# create an instance of PaymentMetadataResponse from a dict
payment_metadata_response_from_dict = PaymentMetadataResponse.from_dict(payment_metadata_response_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


