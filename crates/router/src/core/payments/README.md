A finite state machine is a machine which provides accurate representation of states as per the given state and input. It is used for validation and provide the next state based on given inputs.

In the context of payments, a state machine validates and provides the next state a payment should be in. This would prevent unsupported state transitions to happen, for ex: A state transition from Success to Processing would be prevented by the state machine.

The two states that are to be intercepted by the state machine are

Responsibilities of a finite state machine

- Validate state transition based on given state and event.
- Transition state based on current state and event.

- Payment Intent state machine

```mermaid
flowchart TD

    A{PaymentsAPI} --> |amount,currency| RequiresPaymentMethod

    RequiresPaymentMethod -->|payment_method| RequiresConfirmation
    RequiresConfirmation --> |confirm| Processing

    Processing --> CallConnector{call_connector}

    CallConnector --> |Failure| RetryPossible{auto_retry_possible?}
    RetryPossible --> |yes| Processing
    RetryPossible ---> |no| Failed

    CallConnector --> |Success| AuthType{auth type?}
    AuthType --> |3ds| RequiresCustomerAction
    AuthType --> |no-3ds| AuthUpgrade{auth_upgraded?}
    AuthUpgrade --> |no| CaptureMethod{capture method}
    AuthUpgrade --> |yes| RequiresCustomerAction

    CaptureMethod ---> |manual| RequiresCapture
    CaptureMethod ---> |manual_multiple| RequiresCapture
    CaptureMethod ---> |automatic| Succeeded

    RequiresCustomerAction --> CustomerAction{customer_action}
    CustomerAction --->|success| CaptureMethod
    CustomerAction --->|failure| Failed

    RequiresCapture --> |capture| CaptureCall{capture payment}

    CallConnectorVoid ----> |success| Cancelled
    CallConnectorVoid -----> |failure| Failed

    CaptureCall --> |success| ManualCaptureMethodSuccess{manual_capture_method}
    ManualCaptureMethodSuccess -----> |manual| AmountCapturedCheckManual{amount_captured = authorized_amount?}
    AmountCapturedCheckManual ---> |yes| Succeeded
    AmountCapturedCheckManual ---> |no| PartiallyCaptured

    CaptureCall --> |failure| ManualCaptureMethodFailure{manual_capture_method}
    ManualCaptureMethodFailure --> |manual| Failed
    ManualCaptureMethodFailure --> |manual_multiple| NoStateChange


    ManualCaptureMethodSuccess -----> |manual_multiple| AmountCapturedCheckManualMultiple{amount_capturable > 0?}
    AmountCapturedCheckManualMultiple --> |no| Succeeded
    AmountCapturedCheckManualMultiple --> |yes| PartiallyCapturedAndCapturable

    PartiallyCapturedAndCapturable --> |capture| CaptureCall
    PartiallyCapturedAndCapturable --> |void| CallConnectorVoidCapture{call_connector_void}
    CallConnectorVoidCapture --> |success| PartiallyCaptured
    CallConnectorVoidCapture --> |failure| NoStateChange


    RequiresCapture --> |void| CallConnectorVoid{call_connector_void}
```

```
