# RefundResponse1


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**type** | **str** |  | 
**object** | [**RefundResponse**](RefundResponse.md) |  | 

## Example

```python
from hyperswitch.models.refund_response1 import RefundResponse1

# TODO update the JSON string below
json = "{}"
# create an instance of RefundResponse1 from a JSON string
refund_response1_instance = RefundResponse1.from_json(json)
# print the JSON string representation of the object
print(RefundResponse1.to_json())

# convert the object into a dict
refund_response1_dict = refund_response1_instance.to_dict()
# create an instance of RefundResponse1 from a dict
refund_response1_from_dict = RefundResponse1.from_dict(refund_response1_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


