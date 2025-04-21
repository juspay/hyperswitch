# PaymentsErrorResponse


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**error** | [**PaymentsErrorResponseError**](PaymentsErrorResponseError.md) |  | [optional] 

## Example

```python
from hyperswitch.models.payments_error_response import PaymentsErrorResponse

# TODO update the JSON string below
json = "{}"
# create an instance of PaymentsErrorResponse from a JSON string
payments_error_response_instance = PaymentsErrorResponse.from_json(json)
# print the JSON string representation of the object
print(PaymentsErrorResponse.to_json())

# convert the object into a dict
payments_error_response_dict = payments_error_response_instance.to_dict()
# create an instance of PaymentsErrorResponse from a dict
payments_error_response_from_dict = PaymentsErrorResponse.from_dict(payments_error_response_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


