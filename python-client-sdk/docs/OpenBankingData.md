# OpenBankingData


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**open_banking_pis** | **object** |  | 

## Example

```python
from hyperswitch.models.open_banking_data import OpenBankingData

# TODO update the JSON string below
json = "{}"
# create an instance of OpenBankingData from a JSON string
open_banking_data_instance = OpenBankingData.from_json(json)
# print the JSON string representation of the object
print(OpenBankingData.to_json())

# convert the object into a dict
open_banking_data_dict = open_banking_data_instance.to_dict()
# create an instance of OpenBankingData from a dict
open_banking_data_from_dict = OpenBankingData.from_dict(open_banking_data_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


