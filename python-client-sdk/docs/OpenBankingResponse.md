# OpenBankingResponse


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**open_banking_pis** | **object** |  | 

## Example

```python
from hyperswitch.models.open_banking_response import OpenBankingResponse

# TODO update the JSON string below
json = "{}"
# create an instance of OpenBankingResponse from a JSON string
open_banking_response_instance = OpenBankingResponse.from_json(json)
# print the JSON string representation of the object
print(OpenBankingResponse.to_json())

# convert the object into a dict
open_banking_response_dict = open_banking_response_instance.to_dict()
# create an instance of OpenBankingResponse from a dict
open_banking_response_from_dict = OpenBankingResponse.from_dict(open_banking_response_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


