# PayoutMethodDataResponse

The payout method information for response

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**card** | [**CardAdditionalData**](CardAdditionalData.md) |  | 
**bank** | [**BankAdditionalData**](BankAdditionalData.md) |  | 
**wallet** | [**WalletAdditionalData**](WalletAdditionalData.md) |  | 

## Example

```python
from hyperswitch.models.payout_method_data_response import PayoutMethodDataResponse

# TODO update the JSON string below
json = "{}"
# create an instance of PayoutMethodDataResponse from a JSON string
payout_method_data_response_instance = PayoutMethodDataResponse.from_json(json)
# print the JSON string representation of the object
print(PayoutMethodDataResponse.to_json())

# convert the object into a dict
payout_method_data_response_dict = payout_method_data_response_instance.to_dict()
# create an instance of PayoutMethodDataResponse from a dict
payout_method_data_response_from_dict = PayoutMethodDataResponse.from_dict(payout_method_data_response_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


