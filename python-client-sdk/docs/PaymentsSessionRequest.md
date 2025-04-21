# PaymentsSessionRequest


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**payment_id** | **str** | The identifier for the payment | 
**client_secret** | **str** | This is a token which expires after 15 minutes, used from the client to authenticate and create sessions from the SDK | 
**wallets** | [**List[PaymentMethodType]**](PaymentMethodType.md) | The list of the supported wallets | 
**merchant_connector_details** | [**MerchantConnectorDetailsWrap**](MerchantConnectorDetailsWrap.md) |  | [optional] 

## Example

```python
from hyperswitch.models.payments_session_request import PaymentsSessionRequest

# TODO update the JSON string below
json = "{}"
# create an instance of PaymentsSessionRequest from a JSON string
payments_session_request_instance = PaymentsSessionRequest.from_json(json)
# print the JSON string representation of the object
print(PaymentsSessionRequest.to_json())

# convert the object into a dict
payments_session_request_dict = payments_session_request_instance.to_dict()
# create an instance of PaymentsSessionRequest from a dict
payments_session_request_from_dict = PaymentsSessionRequest.from_dict(payments_session_request_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


