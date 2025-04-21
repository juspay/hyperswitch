# PaymentsCompleteAuthorizeRequest


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**shipping** | [**Address**](Address.md) |  | [optional] 
**client_secret** | **str** | Client Secret | 
**threeds_method_comp_ind** | [**ThreeDsCompletionIndicator**](ThreeDsCompletionIndicator.md) |  | [optional] 

## Example

```python
from hyperswitch.models.payments_complete_authorize_request import PaymentsCompleteAuthorizeRequest

# TODO update the JSON string below
json = "{}"
# create an instance of PaymentsCompleteAuthorizeRequest from a JSON string
payments_complete_authorize_request_instance = PaymentsCompleteAuthorizeRequest.from_json(json)
# print the JSON string representation of the object
print(PaymentsCompleteAuthorizeRequest.to_json())

# convert the object into a dict
payments_complete_authorize_request_dict = payments_complete_authorize_request_instance.to_dict()
# create an instance of PaymentsCompleteAuthorizeRequest from a dict
payments_complete_authorize_request_from_dict = PaymentsCompleteAuthorizeRequest.from_dict(payments_complete_authorize_request_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


