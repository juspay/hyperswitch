# OpenBankingSessionToken


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**open_banking_session_token** | **str** | The session token for OpenBanking Connectors | 

## Example

```python
from hyperswitch.models.open_banking_session_token import OpenBankingSessionToken

# TODO update the JSON string below
json = "{}"
# create an instance of OpenBankingSessionToken from a JSON string
open_banking_session_token_instance = OpenBankingSessionToken.from_json(json)
# print the JSON string representation of the object
print(OpenBankingSessionToken.to_json())

# convert the object into a dict
open_banking_session_token_dict = open_banking_session_token_instance.to_dict()
# create an instance of OpenBankingSessionToken from a dict
open_banking_session_token_from_dict = OpenBankingSessionToken.from_dict(open_banking_session_token_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


