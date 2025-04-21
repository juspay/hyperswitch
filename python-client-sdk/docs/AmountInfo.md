# AmountInfo


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**label** | **str** | The label must be the name of the merchant. | 
**type** | **str** | A value that indicates whether the line item(Ex: total, tax, discount, or grand total) is final or pending. | [optional] 
**amount** | **str** | The total amount for the payment in majot unit string (Ex: 38.02) | 

## Example

```python
from hyperswitch.models.amount_info import AmountInfo

# TODO update the JSON string below
json = "{}"
# create an instance of AmountInfo from a JSON string
amount_info_instance = AmountInfo.from_json(json)
# print the JSON string representation of the object
print(AmountInfo.to_json())

# convert the object into a dict
amount_info_dict = amount_info_instance.to_dict()
# create an instance of AmountInfo from a dict
amount_info_from_dict = AmountInfo.from_dict(amount_info_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


