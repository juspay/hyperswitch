# PayLaterDataOneOfKlarnaRedirect

For KlarnaRedirect as PayLater Option

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**billing_email** | **str** | The billing email | [optional] 
**billing_country** | [**CountryAlpha2**](CountryAlpha2.md) |  | [optional] 

## Example

```python
from hyperswitch.models.pay_later_data_one_of_klarna_redirect import PayLaterDataOneOfKlarnaRedirect

# TODO update the JSON string below
json = "{}"
# create an instance of PayLaterDataOneOfKlarnaRedirect from a JSON string
pay_later_data_one_of_klarna_redirect_instance = PayLaterDataOneOfKlarnaRedirect.from_json(json)
# print the JSON string representation of the object
print(PayLaterDataOneOfKlarnaRedirect.to_json())

# convert the object into a dict
pay_later_data_one_of_klarna_redirect_dict = pay_later_data_one_of_klarna_redirect_instance.to_dict()
# create an instance of PayLaterDataOneOfKlarnaRedirect from a dict
pay_later_data_one_of_klarna_redirect_from_dict = PayLaterDataOneOfKlarnaRedirect.from_dict(pay_later_data_one_of_klarna_redirect_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


