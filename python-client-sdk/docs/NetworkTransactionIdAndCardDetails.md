# NetworkTransactionIdAndCardDetails


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**card_number** | **str** | The card number | 
**card_exp_month** | **str** | The card&#39;s expiry month | 
**card_exp_year** | **str** | The card&#39;s expiry year | 
**card_holder_name** | **str** | The card holder&#39;s name | 
**card_issuer** | **str** | The name of the issuer of card | [optional] 
**card_network** | [**CardNetwork**](CardNetwork.md) |  | [optional] 
**card_type** | **str** |  | [optional] 
**card_issuing_country** | **str** |  | [optional] 
**bank_code** | **str** |  | [optional] 
**nick_name** | **str** | The card holder&#39;s nick name | [optional] 
**network_transaction_id** | **str** | The network transaction ID provided by the card network during a CIT (Customer Initiated Transaction), where &#x60;setup_future_usage&#x60; is set to &#x60;off_session&#x60;. | 

## Example

```python
from hyperswitch.models.network_transaction_id_and_card_details import NetworkTransactionIdAndCardDetails

# TODO update the JSON string below
json = "{}"
# create an instance of NetworkTransactionIdAndCardDetails from a JSON string
network_transaction_id_and_card_details_instance = NetworkTransactionIdAndCardDetails.from_json(json)
# print the JSON string representation of the object
print(NetworkTransactionIdAndCardDetails.to_json())

# convert the object into a dict
network_transaction_id_and_card_details_dict = network_transaction_id_and_card_details_instance.to_dict()
# create an instance of NetworkTransactionIdAndCardDetails from a dict
network_transaction_id_and_card_details_from_dict = NetworkTransactionIdAndCardDetails.from_dict(network_transaction_id_and_card_details_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


