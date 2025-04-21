# PaymentsIncrementalAuthorizationRequest


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**amount** | **int** | The total amount including previously authorized amount and additional amount | 
**reason** | **str** | Reason for incremental authorization | [optional] 

## Example

```python
from hyperswitch.models.payments_incremental_authorization_request import PaymentsIncrementalAuthorizationRequest

# TODO update the JSON string below
json = "{}"
# create an instance of PaymentsIncrementalAuthorizationRequest from a JSON string
payments_incremental_authorization_request_instance = PaymentsIncrementalAuthorizationRequest.from_json(json)
# print the JSON string representation of the object
print(PaymentsIncrementalAuthorizationRequest.to_json())

# convert the object into a dict
payments_incremental_authorization_request_dict = payments_incremental_authorization_request_instance.to_dict()
# create an instance of PaymentsIncrementalAuthorizationRequest from a dict
payments_incremental_authorization_request_from_dict = PaymentsIncrementalAuthorizationRequest.from_dict(payments_incremental_authorization_request_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


