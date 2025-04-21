# PayoutMethodDataResponseOneOf1


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**bank** | [**BankAdditionalData**](BankAdditionalData.md) |  | 

## Example

```python
from hyperswitch.models.payout_method_data_response_one_of1 import PayoutMethodDataResponseOneOf1

# TODO update the JSON string below
json = "{}"
# create an instance of PayoutMethodDataResponseOneOf1 from a JSON string
payout_method_data_response_one_of1_instance = PayoutMethodDataResponseOneOf1.from_json(json)
# print the JSON string representation of the object
print(PayoutMethodDataResponseOneOf1.to_json())

# convert the object into a dict
payout_method_data_response_one_of1_dict = payout_method_data_response_one_of1_instance.to_dict()
# create an instance of PayoutMethodDataResponseOneOf1 from a dict
payout_method_data_response_one_of1_from_dict = PayoutMethodDataResponseOneOf1.from_dict(payout_method_data_response_one_of1_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


