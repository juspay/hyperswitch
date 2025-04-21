# PaymentsPostSessionTokensResponse


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**payment_id** | **str** | The identifier for the payment | 
**next_action** | [**NextActionData**](NextActionData.md) |  | [optional] 
**status** | [**IntentStatus**](IntentStatus.md) |  | 

## Example

```python
from hyperswitch.models.payments_post_session_tokens_response import PaymentsPostSessionTokensResponse

# TODO update the JSON string below
json = "{}"
# create an instance of PaymentsPostSessionTokensResponse from a JSON string
payments_post_session_tokens_response_instance = PaymentsPostSessionTokensResponse.from_json(json)
# print the JSON string representation of the object
print(PaymentsPostSessionTokensResponse.to_json())

# convert the object into a dict
payments_post_session_tokens_response_dict = payments_post_session_tokens_response_instance.to_dict()
# create an instance of PaymentsPostSessionTokensResponse from a dict
payments_post_session_tokens_response_from_dict = PaymentsPostSessionTokensResponse.from_dict(payments_post_session_tokens_response_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


