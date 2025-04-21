# PaypalRedirection


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**email** | **str** | paypal&#39;s email address | [optional] 

## Example

```python
from hyperswitch.models.paypal_redirection import PaypalRedirection

# TODO update the JSON string below
json = "{}"
# create an instance of PaypalRedirection from a JSON string
paypal_redirection_instance = PaypalRedirection.from_json(json)
# print the JSON string representation of the object
print(PaypalRedirection.to_json())

# convert the object into a dict
paypal_redirection_dict = paypal_redirection_instance.to_dict()
# create an instance of PaypalRedirection from a dict
paypal_redirection_from_dict = PaypalRedirection.from_dict(paypal_redirection_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


