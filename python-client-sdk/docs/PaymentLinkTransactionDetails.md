# PaymentLinkTransactionDetails


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**key** | **str** | Key for the transaction details | 
**value** | **str** | Value for the transaction details | 
**ui_configuration** | [**TransactionDetailsUiConfiguration**](TransactionDetailsUiConfiguration.md) |  | [optional] 

## Example

```python
from hyperswitch.models.payment_link_transaction_details import PaymentLinkTransactionDetails

# TODO update the JSON string below
json = "{}"
# create an instance of PaymentLinkTransactionDetails from a JSON string
payment_link_transaction_details_instance = PaymentLinkTransactionDetails.from_json(json)
# print the JSON string representation of the object
print(PaymentLinkTransactionDetails.to_json())

# convert the object into a dict
payment_link_transaction_details_dict = payment_link_transaction_details_instance.to_dict()
# create an instance of PaymentLinkTransactionDetails from a dict
payment_link_transaction_details_from_dict = PaymentLinkTransactionDetails.from_dict(payment_link_transaction_details_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


