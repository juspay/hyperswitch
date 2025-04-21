# BankCodeResponse


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**bank_name** | [**List[BankNames]**](BankNames.md) |  | 
**eligible_connectors** | **List[str]** |  | 

## Example

```python
from hyperswitch.models.bank_code_response import BankCodeResponse

# TODO update the JSON string below
json = "{}"
# create an instance of BankCodeResponse from a JSON string
bank_code_response_instance = BankCodeResponse.from_json(json)
# print the JSON string representation of the object
print(BankCodeResponse.to_json())

# convert the object into a dict
bank_code_response_dict = bank_code_response_instance.to_dict()
# create an instance of BankCodeResponse from a dict
bank_code_response_from_dict = BankCodeResponse.from_dict(bank_code_response_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


