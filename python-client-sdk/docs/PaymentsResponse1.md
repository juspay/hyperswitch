# PaymentsResponse1


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**type** | **str** |  | 
**object** | [**PaymentsResponse**](PaymentsResponse.md) |  | 

## Example

```python
from hyperswitch.models.payments_response1 import PaymentsResponse1

# TODO update the JSON string below
json = "{}"
# create an instance of PaymentsResponse1 from a JSON string
payments_response1_instance = PaymentsResponse1.from_json(json)
# print the JSON string representation of the object
print(PaymentsResponse1.to_json())

# convert the object into a dict
payments_response1_dict = payments_response1_instance.to_dict()
# create an instance of PaymentsResponse1 from a dict
payments_response1_from_dict = PaymentsResponse1.from_dict(payments_response1_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


