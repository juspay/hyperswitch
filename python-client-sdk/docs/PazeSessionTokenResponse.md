# PazeSessionTokenResponse


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**client_id** | **str** | Paze Client ID | 
**client_name** | **str** | Client Name to be displayed on the Paze screen | 
**client_profile_id** | **str** | Paze Client Profile ID | 
**transaction_currency_code** | [**Currency**](Currency.md) |  | 
**transaction_amount** | **str** | The transaction amount | 
**email_address** | **str** | Email Address | [optional] 

## Example

```python
from hyperswitch.models.paze_session_token_response import PazeSessionTokenResponse

# TODO update the JSON string below
json = "{}"
# create an instance of PazeSessionTokenResponse from a JSON string
paze_session_token_response_instance = PazeSessionTokenResponse.from_json(json)
# print the JSON string representation of the object
print(PazeSessionTokenResponse.to_json())

# convert the object into a dict
paze_session_token_response_dict = paze_session_token_response_instance.to_dict()
# create an instance of PazeSessionTokenResponse from a dict
paze_session_token_response_from_dict = PazeSessionTokenResponse.from_dict(paze_session_token_response_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


