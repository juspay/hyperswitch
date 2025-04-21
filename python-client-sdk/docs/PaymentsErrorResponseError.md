# PaymentsErrorResponseError


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**message** | **str** | Error message | [optional] 
**code** | **str** | Error code | [optional] 

## Example

```python
from hyperswitch.models.payments_error_response_error import PaymentsErrorResponseError

# TODO update the JSON string below
json = "{}"
# create an instance of PaymentsErrorResponseError from a JSON string
payments_error_response_error_instance = PaymentsErrorResponseError.from_json(json)
# print the JSON string representation of the object
print(PaymentsErrorResponseError.to_json())

# convert the object into a dict
payments_error_response_error_dict = payments_error_response_error_instance.to_dict()
# create an instance of PaymentsErrorResponseError from a dict
payments_error_response_error_from_dict = PaymentsErrorResponseError.from_dict(payments_error_response_error_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


