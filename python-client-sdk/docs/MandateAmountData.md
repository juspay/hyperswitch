# MandateAmountData


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**amount** | **int** | The maximum amount to be debited for the mandate transaction | 
**currency** | [**Currency**](Currency.md) |  | 
**start_date** | **datetime** | Specifying start date of the mandate | [optional] 
**end_date** | **datetime** | Specifying end date of the mandate | [optional] 
**metadata** | **object** | Additional details required by mandate | [optional] 

## Example

```python
from hyperswitch.models.mandate_amount_data import MandateAmountData

# TODO update the JSON string below
json = "{}"
# create an instance of MandateAmountData from a JSON string
mandate_amount_data_instance = MandateAmountData.from_json(json)
# print the JSON string representation of the object
print(MandateAmountData.to_json())

# convert the object into a dict
mandate_amount_data_dict = mandate_amount_data_instance.to_dict()
# create an instance of MandateAmountData from a dict
mandate_amount_data_from_dict = MandateAmountData.from_dict(mandate_amount_data_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


