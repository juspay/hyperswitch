# Hyperswitch Detailed Sequence Diagrams

---
**Last Updated:** 2025-05-27  
**Documentation Status:** Complete
---

This document provides detailed sequence diagrams for complex flows in the Hyperswitch payment orchestration platform, illustrating the step-by-step interactions for specific scenarios.

## 3D Secure (3DS) Authentication Flow

The following diagram illustrates the detailed sequence of interactions during a 3DS authentication flow:

```mermaid
sequenceDiagram
    participant MA as Merchant App
    participant R as Router API
    participant Core as Payment Core
    participant Conn as Connector Interface
    participant DB as Database
    participant PP as Payment Processor
    participant 3DS as 3DS Provider
    participant Customer as Customer Browser
    
    MA->>R: POST /payments (with payment_method)
    R->>Core: Process Payment Request
    Core->>DB: Create Payment Record
    Core->>Core: Apply Routing Strategy
    Core->>Conn: Initiate Payment
    
    Conn->>PP: Process Payment
    PP->>PP: Determine 3DS Required
    PP-->>Conn: Return 3DS Required (with redirect URL)
    Conn-->>Core: Return 3DS Required Response
    
    Core->>DB: Update Payment Status (requires_authentication)
    Core-->>R: Return 3DS Required Response
    R-->>MA: Return 3DS Required Response
    
    MA->>Customer: Redirect to 3DS Authentication URL
    Customer->>3DS: Complete 3DS Challenge
    3DS->>PP: Submit 3DS Authentication Result
    PP-->>3DS: Acknowledge Result
    3DS-->>Customer: Redirect to Return URL
    
    alt 3DS Webhook Path
        PP->>R: Send Authentication Webhook
        R->>Core: Process Authentication Webhook
        Core->>DB: Update Payment Status
        Core->>Conn: Continue Payment Processing
        Conn->>PP: Continue Payment Processing
        PP-->>Conn: Final Payment Result
        Conn-->>Core: Processed Result
        Core->>DB: Update Payment Status
        Core-->>R: Webhook Response
        R-->>PP: 200 OK
    else 3DS Client Polling Path
        Customer->>MA: Redirect with Authentication Result
        MA->>R: GET /payments/{payment_id}
        R->>Core: Get Payment Status
        Core->>DB: Retrieve Payment Status
        
        alt Payment Status Not Updated Yet
            Core->>Conn: Check Payment Status
            Conn->>PP: Get Payment Status
            PP-->>Conn: Current Status
            Conn-->>Core: Current Status
            Core->>DB: Update Payment Status
        end
        
        Core-->>R: Return Payment Status
        R-->>MA: Return Payment Status
    end
    
    MA->>Customer: Display Payment Result
```

## Payment Routing with Fallback Mechanism

The following diagram illustrates the payment routing process with fallback to alternative connectors when the primary connector fails:

```mermaid
sequenceDiagram
    participant MA as Merchant App
    participant R as Router API
    participant Core as Payment Core
    participant RE as Routing Engine
    participant Conn as Connector Interface
    participant DB as Database
    participant Redis as Redis Cache
    participant PP1 as Primary Processor
    participant PP2 as Secondary Processor
    
    MA->>R: POST /payments
    R->>Core: Process Payment Request
    
    Core->>DB: Retrieve Merchant Configuration
    DB-->>Core: Merchant Configuration
    
    Core->>Redis: Get Routing Rules & Connector Performance
    Redis-->>Core: Routing Rules & Connector Performance
    
    Core->>RE: Determine Optimal Connector
    
    RE->>RE: Apply Routing Strategy
    Note over RE: Consider:<br/>- Success rates<br/>- Cost<br/>- Features required<br/>- Geographic factors
    
    RE-->>Core: Selected Primary & Fallback Connectors
    
    Core->>Conn: Process with Primary Connector (PP1)
    Conn->>PP1: Process Payment
    
    alt Primary Connector Succeeds
        PP1-->>Conn: Success Response
        Conn-->>Core: Payment Successful
        Core->>Redis: Update Connector Metrics (success)
    else Primary Connector Fails
        PP1-->>Conn: Error Response
        Conn-->>Core: Primary Connector Failed
        Core->>Redis: Update Connector Metrics (failure)
        Core->>DB: Log Connector Failure
        
        Note over Core: Fallback Decision
        Core->>Conn: Process with Fallback Connector (PP2)
        Conn->>PP2: Process Payment
        
        alt Fallback Connector Succeeds
            PP2-->>Conn: Success Response
            Conn-->>Core: Payment Successful
            Core->>Redis: Update Connector Metrics (fallback success)
        else Fallback Connector Fails
            PP2-->>Conn: Error Response
            Conn-->>Core: Fallback Connector Failed
            Core->>Redis: Update Connector Metrics (fallback failure)
            
            Note over Core: Final Failure Handling
            Core->>DB: Update Payment as Failed
            Core-->>R: Payment Failed Response
            R-->>MA: Payment Failed Response
        end
    end
    
    Core->>DB: Update Payment Status
    Core-->>R: Payment Response
    R-->>MA: Payment Response
```

## Webhook Processing Sequence

The following diagram illustrates the detailed sequence for processing webhooks from payment processors:

```mermaid
sequenceDiagram
    participant PP as Payment Processor
    participant R as Router API (Webhook Endpoint)
    participant Auth as Authentication Middleware
    participant WH as Webhook Handler
    participant Trans as Transformer
    participant Core as Payment Core
    participant DB as Database
    participant MA as Merchant App
    
    PP->>R: POST /webhooks/{connector}
    R->>Auth: Validate Webhook Source
    
    Auth->>Auth: Verify IP Whitelist
    Auth->>Auth: Verify Webhook Signature
    
    Auth->>WH: Forward Verified Webhook
    
    WH->>DB: Log Raw Webhook
    WH->>Trans: Transform Webhook Payload
    
    Trans->>Trans: Normalize to Internal Format
    Trans-->>WH: Normalized Event Data
    
    WH->>DB: Retrieve Associated Payment/Refund
    DB-->>WH: Payment/Refund Details
    
    WH->>WH: Validate Event Against Current State
    
    alt Valid State Transition
        WH->>Core: Update Payment/Refund Status
        Core->>DB: Update Status in Database
        Core-->>WH: Status Updated
        
        alt Merchant Notification Required
            WH->>WH: Generate Merchant Event
            WH->>MA: Send Event to Merchant Webhook URL
            MA-->>WH: Acknowledge (200 OK)
        end
        
        WH->>DB: Log Webhook Processing Complete
        WH-->>R: Processing Success (200 OK)
        R-->>PP: 200 OK
    else Invalid State Transition
        WH->>DB: Log Validation Error
        WH->>WH: Determine If Retry Needed
        
        alt Retry Needed
            WH->>DB: Schedule Retry
            WH-->>R: Temporary Error (500)
            R-->>PP: 500 Error
        else Permanent Failure
            WH->>DB: Log Permanent Failure
            WH-->>R: Success (200 OK - Prevent Retries)
            R-->>PP: 200 OK
        end
    end
```

## Payment Capture Sequence

The following diagram illustrates the capture flow for payments that use separate authorization and capture steps:

```mermaid
sequenceDiagram
    participant MA as Merchant App
    participant R as Router API
    participant Core as Payment Core
    participant Conn as Connector Interface
    participant DB as Database
    participant PP as Payment Processor
    
    MA->>R: POST /payments/{payment_id}/capture
    R->>Core: Process Capture Request
    
    Core->>DB: Retrieve Payment Details
    DB-->>Core: Payment Details
    
    alt Payment Not in Authorized State
        Core-->>R: Error - Invalid Payment State
        R-->>MA: Error Response
    else Payment in Authorized State
        Core->>Core: Validate Capture Amount
        
        alt Validation Failed
            Core-->>R: Error - Invalid Capture Amount
            R-->>MA: Error Response
        else Validation Passed
            Core->>DB: Update Payment Status (capture_initiated)
            
            Core->>Conn: Initiate Capture
            Conn->>PP: Process Capture
            
            alt Capture Successful
                PP-->>Conn: Capture Success
                Conn-->>Core: Capture Success
                Core->>DB: Update Payment Status (captured)
                Core-->>R: Capture Success
                R-->>MA: Capture Success Response
            else Capture Failed
                PP-->>Conn: Capture Failed
                Conn-->>Core: Capture Failed
                Core->>DB: Update Payment Status (capture_failed)
                Core-->>R: Capture Failed
                R-->>MA: Capture Failed Response
            end
        end
    end
```

## Scheduled Task Execution Sequence

The following diagram illustrates the sequence for executing scheduled tasks via the Scheduler component:

```mermaid
sequenceDiagram
    participant Producer as Scheduler (Producer)
    participant Redis as Redis Queue
    participant Consumer as Scheduler (Consumer)
    participant Core as Router Core
    participant DB as Database
    
    loop Every Scheduler Interval
        Producer->>DB: Query for Due Tasks
        DB-->>Producer: Tasks Due for Execution
        
        loop For Each Batch of Tasks
            Producer->>Producer: Group Tasks by Type
            Producer->>Redis: Enqueue Task Batch
            Note over Redis: Tasks stored with:<br/>- Task ID<br/>- Task Type<br/>- Parameters<br/>- Retry Count<br/>- Business Data
        end
    end
    
    loop Continuous Polling
        Consumer->>Redis: Poll for Tasks
        Redis-->>Consumer: Next Batch of Tasks
        
        loop For Each Task in Batch
            Consumer->>Consumer: Deserialize Task
            Consumer->>Core: Execute Task Action
            
            alt Task Execution Successful
                Core->>DB: Update Task Status
                Core-->>Consumer: Success Result
                Consumer->>DB: Mark Task Complete
            else Task Execution Failed
                Core-->>Consumer: Error Result
                
                alt Max Retries Not Reached
                    Consumer->>Consumer: Calculate Backoff
                    Consumer->>Redis: Requeue with Incremented Retry Count
                else Max Retries Reached
                    Consumer->>DB: Mark Task Failed
                    Consumer->>DB: Log Failure Details
                end
            end
        end
    end
```

## Key Implementation Notes

### 3DS Flow Implementation
- The system supports both synchronous and asynchronous 3DS flows
- Webhook processing is the primary mechanism for updating 3DS authentication results
- Client-side polling serves as a fallback mechanism
- The system maintains payment state through the entire 3DS redirect flow

### Routing Mechanism Implementation
- Routing decisions consider multiple factors including success rates, costs, and features
- Success and failure metrics are continuously updated for each connector
- Fallback mechanisms can include multiple levels of fallbacks with conditional logic
- Connector selection is logged for auditing and analysis

### Webhook Processing Implementation
- All webhooks are verified for authenticity before processing
- Raw webhook data is logged before transformation
- Events are normalized to a standard internal format independent of the source connector
- State validation ensures consistency during asynchronous processing
- Merchant notifications are sent only after successful internal processing

### Security Considerations
- All sensitive data is tokenized through the Locker component (not shown in diagrams for simplicity)
- Authentication is verified at multiple layers
- Webhook signatures are validated cryptographically
- IP whitelisting provides additional security for webhook endpoints

## See Also
- [System Architecture Diagram](./system_architecture_diagram.md)
- [Component Interaction Diagram](./component_interaction_diagram.md)
- [Data Flow Diagram](./data_flow_diagram.md)
- [State Transition Diagram](./state_transition_diagram.md)
