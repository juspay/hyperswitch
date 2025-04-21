# PayLaterDataOneOf


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**klarna_redirect** | [**PayLaterDataOneOfKlarnaRedirect**](PayLaterDataOneOfKlarnaRedirect.md) |  | 

## Example

```python
from hyperswitch.models.pay_later_data_one_of import PayLaterDataOneOf

# TODO update the JSON string below
json = "{}"
# create an instance of PayLaterDataOneOf from a JSON string
pay_later_data_one_of_instance = PayLaterDataOneOf.from_json(json)
# print the JSON string representation of the object
print(PayLaterDataOneOf.to_json())

# convert the object into a dict
pay_later_data_one_of_dict = pay_later_data_one_of_instance.to_dict()
# create an instance of PayLaterDataOneOf from a dict
pay_later_data_one_of_from_dict = PayLaterDataOneOf.from_dict(pay_later_data_one_of_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


