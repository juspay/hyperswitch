Currently in hyperswitch, under authorize flow, there can be a lot of flows that must be called before actually calling the authorize. Like session/access token, customer create, order create etc. 
Currently these are scattered all across the handler function 'pub async fn payments_operation_core<F, Req, Op, FData, D>(' .
I want to standardize the flows like this.

PrimaryFlows and Secondary Flows.
PrimaryFlows: The actual flow.
SecondaryFlow: These flows might come as prerequisites before the Primary Flows. There can be multiple Secondary Flows for a PrimaryFlow.

PrimaryFlows can be defined as:
    1. A flow where the response is returned to the client.
        Eg: Authorize, Capture
        Authorize can have SessionToken, OrderCreate etc.

If Authorize is a PrimaryFlow, Then SessionTokena and OrderCreate will be SecondaryFlow.
The order will be like SessionToken(2ndary) -> OrderCreate(2ndary) -> Authorize(Primary).
Similarly, 
* SessionToken(2ndary) -> PreAuthN(Primary).
* SessionToken(2ndary) -> AuthN(Primary).
* PostAuthN(2ndary) -> Authorization(Primary)

Lets have marker trait for Primary and Secondary Flows. Feel free to come up with a better nomenclature for things.

Execution order must be known at compile time.