# TransactionDetailsUiConfiguration


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**position** | **int** | Position of the key-value pair in the UI | [optional] 
**is_key_bold** | **bool** | Whether the key should be bold | [optional] [default to False]
**is_value_bold** | **bool** | Whether the value should be bold | [optional] [default to False]

## Example

```python
from hyperswitch.models.transaction_details_ui_configuration import TransactionDetailsUiConfiguration

# TODO update the JSON string below
json = "{}"
# create an instance of TransactionDetailsUiConfiguration from a JSON string
transaction_details_ui_configuration_instance = TransactionDetailsUiConfiguration.from_json(json)
# print the JSON string representation of the object
print(TransactionDetailsUiConfiguration.to_json())

# convert the object into a dict
transaction_details_ui_configuration_dict = transaction_details_ui_configuration_instance.to_dict()
# create an instance of TransactionDetailsUiConfiguration from a dict
transaction_details_ui_configuration_from_dict = TransactionDetailsUiConfiguration.from_dict(transaction_details_ui_configuration_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


