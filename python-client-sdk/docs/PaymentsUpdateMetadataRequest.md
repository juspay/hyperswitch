# PaymentsUpdateMetadataRequest


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**metadata** | **object** | Metadata is useful for storing additional, unstructured information on an object. | 

## Example

```python
from hyperswitch.models.payments_update_metadata_request import PaymentsUpdateMetadataRequest

# TODO update the JSON string below
json = "{}"
# create an instance of PaymentsUpdateMetadataRequest from a JSON string
payments_update_metadata_request_instance = PaymentsUpdateMetadataRequest.from_json(json)
# print the JSON string representation of the object
print(PaymentsUpdateMetadataRequest.to_json())

# convert the object into a dict
payments_update_metadata_request_dict = payments_update_metadata_request_instance.to_dict()
# create an instance of PaymentsUpdateMetadataRequest from a dict
payments_update_metadata_request_from_dict = PaymentsUpdateMetadataRequest.from_dict(payments_update_metadata_request_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


