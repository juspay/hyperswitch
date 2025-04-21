# PaymentsUpdateMetadataResponse


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**payment_id** | **str** | The identifier for the payment | 
**metadata** | **object** | Metadata is useful for storing additional, unstructured information on an object. | [optional] 

## Example

```python
from hyperswitch.models.payments_update_metadata_response import PaymentsUpdateMetadataResponse

# TODO update the JSON string below
json = "{}"
# create an instance of PaymentsUpdateMetadataResponse from a JSON string
payments_update_metadata_response_instance = PaymentsUpdateMetadataResponse.from_json(json)
# print the JSON string representation of the object
print(PaymentsUpdateMetadataResponse.to_json())

# convert the object into a dict
payments_update_metadata_response_dict = payments_update_metadata_response_instance.to_dict()
# create an instance of PaymentsUpdateMetadataResponse from a dict
payments_update_metadata_response_from_dict = PaymentsUpdateMetadataResponse.from_dict(payments_update_metadata_response_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


