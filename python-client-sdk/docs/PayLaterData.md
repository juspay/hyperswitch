# PayLaterData


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**klarna_redirect** | [**PayLaterDataOneOfKlarnaRedirect**](PayLaterDataOneOfKlarnaRedirect.md) |  | 
**klarna_sdk** | [**PayLaterDataOneOf1KlarnaSdk**](PayLaterDataOneOf1KlarnaSdk.md) |  | 
**affirm_redirect** | **object** | For Affirm redirect as PayLater Option | 
**afterpay_clearpay_redirect** | [**PayLaterDataOneOf3AfterpayClearpayRedirect**](PayLaterDataOneOf3AfterpayClearpayRedirect.md) |  | 
**pay_bright_redirect** | **object** | For PayBright Redirect as PayLater Option | 
**walley_redirect** | **object** | For WalleyRedirect as PayLater Option | 
**alma_redirect** | **object** | For Alma Redirection as PayLater Option | 
**atome_redirect** | **object** |  | 

## Example

```python
from hyperswitch.models.pay_later_data import PayLaterData

# TODO update the JSON string below
json = "{}"
# create an instance of PayLaterData from a JSON string
pay_later_data_instance = PayLaterData.from_json(json)
# print the JSON string representation of the object
print(PayLaterData.to_json())

# convert the object into a dict
pay_later_data_dict = pay_later_data_instance.to_dict()
# create an instance of PayLaterData from a dict
pay_later_data_from_dict = PayLaterData.from_dict(pay_later_data_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


