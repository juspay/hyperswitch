# FutureUsage

Indicates that you intend to make future payments with the payment methods used for this Payment. Providing this parameter will attach the payment method to the Customer, if present, after the Payment is confirmed and any required actions from the user are complete. - On_session - Payment method saved only at hyperswitch when consent is provided by the user. CVV will asked during the returning user payment - Off_session - Payment method saved at both hyperswitch and Processor when consent is provided by the user. No input is required during the returning user payment.

## Enum

* `OFF_SESSION` (value: `'off_session'`)

* `ON_SESSION` (value: `'on_session'`)

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


