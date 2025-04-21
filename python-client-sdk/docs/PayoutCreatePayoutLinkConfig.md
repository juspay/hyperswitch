# PayoutCreatePayoutLinkConfig

Custom payout link config for the particular payout, if payout link is to be generated.

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**logo** | **str** | Merchant&#39;s display logo | [optional] 
**merchant_name** | **str** | Custom merchant name for the link | [optional] 
**theme** | **str** | Primary color to be used in the form represented in hex format | [optional] 
**payout_link_id** | **str** | The unique identifier for the collect link. | [optional] 
**enabled_payment_methods** | [**List[EnabledPaymentMethod]**](EnabledPaymentMethod.md) | List of payout methods shown on collect UI | [optional] 
**form_layout** | [**UIWidgetFormLayout**](UIWidgetFormLayout.md) |  | [optional] 
**test_mode** | **bool** | &#x60;test_mode&#x60; allows for opening payout links without any restrictions. This removes - domain name validations - check for making sure link is accessed within an iframe | [optional] 

## Example

```python
from hyperswitch.models.payout_create_payout_link_config import PayoutCreatePayoutLinkConfig

# TODO update the JSON string below
json = "{}"
# create an instance of PayoutCreatePayoutLinkConfig from a JSON string
payout_create_payout_link_config_instance = PayoutCreatePayoutLinkConfig.from_json(json)
# print the JSON string representation of the object
print(PayoutCreatePayoutLinkConfig.to_json())

# convert the object into a dict
payout_create_payout_link_config_dict = payout_create_payout_link_config_instance.to_dict()
# create an instance of PayoutCreatePayoutLinkConfig from a dict
payout_create_payout_link_config_from_dict = PayoutCreatePayoutLinkConfig.from_dict(payout_create_payout_link_config_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


