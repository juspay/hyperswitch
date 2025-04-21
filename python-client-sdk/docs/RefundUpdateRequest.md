# RefundUpdateRequest


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**reason** | **str** | An arbitrary string attached to the object. Often useful for displaying to users and your customer support executive | [optional] 
**metadata** | **object** | You can specify up to 50 keys, with key names up to 40 characters long and values up to 500 characters long. Metadata is useful for storing additional, structured information on an object. | [optional] 

## Example

```python
from hyperswitch.models.refund_update_request import RefundUpdateRequest

# TODO update the JSON string below
json = "{}"
# create an instance of RefundUpdateRequest from a JSON string
refund_update_request_instance = RefundUpdateRequest.from_json(json)
# print the JSON string representation of the object
print(RefundUpdateRequest.to_json())

# convert the object into a dict
refund_update_request_dict = refund_update_request_instance.to_dict()
# create an instance of RefundUpdateRequest from a dict
refund_update_request_from_dict = RefundUpdateRequest.from_dict(refund_update_request_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


