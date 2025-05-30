# Hyperswitch Component Interaction Diagram

---
**Last Updated:** 2025-05-27  
**Documentation Status:** Complete
---

This document provides a detailed component interaction diagram showing how the major components of the Hyperswitch system interact with each other during key operations.

## Component Interaction Overview

The following diagram illustrates the interactions between major components during payment processing:

```mermaid
sequenceDiagram
    participant MA as Merchant Application
    participant RM as Router (Middleware)
    participant RR as Router (Routes)
    participant RC as Router (Core)
    participant RCN as Router (Connector)
    participant DB as PostgreSQL
    participant Cache as Redis
    participant PP as Payment Processor
    participant L as Locker
    
    MA->>RM: API Request (Payment)
    Note over RM: Authentication<br/>Authorization<br/>Request Validation
    RM->>RR: Validated Request
    RR->>RC: Process Payment
    
    RC->>DB: Check Customer & Merchant
    DB-->>RC: Data Retrieved
    
    RC->>Cache: Check Rate Limits & Config
    Cache-->>RC: Config Retrieved
    
    alt Needs Routing Decision
        RC->>RC: Apply Routing Strategy
        Note over RC: Rule-based, Success-rate,<br/>Cost optimization, etc.
    end
    
    RC->>RCN: Forward to Selected Connector
    
    alt Contains PII/PCI Data
        RCN->>L: Store Sensitive Data
        L-->>RCN: Return Token/Reference
    end
    
    RCN->>PP: Process Payment Request
    
    alt Synchronous Response
        PP-->>RCN: Immediate Response
        RCN-->>RC: Processed Response
    else Asynchronous Response
        PP-->>RCN: Acknowledgement
        RCN-->>RC: Pending Status
        Note over PP,RCN: Later webhook notification
    end
    
    RC->>DB: Store Transaction Data
    RC-->>RR: Payment Result
    RR-->>RM: Format API Response
    RM-->>MA: API Response
```

## Refund Flow Component Interaction

The following diagram shows component interactions during the refund process:

```mermaid
sequenceDiagram
    participant MA as Merchant Application
    participant RR as Router (Routes)
    participant RC as Router (Core)
    participant RCN as Router (Connector)
    participant DB as PostgreSQL
    participant PP as Payment Processor
    
    MA->>RR: Refund Request
    RR->>RC: Process Refund
    
    RC->>DB: Verify Original Payment
    DB-->>RC: Payment Details
    
    alt Invalid Payment Status
        RC-->>RR: Error: Invalid Payment
        RR-->>MA: Error Response
    else Valid Payment
        RC->>RCN: Forward Refund to Connector
        RCN->>PP: Process Refund
        PP-->>RCN: Refund Response
        RCN-->>RC: Processed Response
        RC->>DB: Update Transaction Status
        RC-->>RR: Refund Result
        RR-->>MA: API Response
    end
```

## Webhook Processing Component Interaction

The following diagram illustrates the component interactions during webhook processing:

```mermaid
sequenceDiagram
    participant PP as Payment Processor
    participant RM as Router (Middleware)
    participant RR as Router (Routes)
    participant RC as Router (Core/Webhooks)
    participant DB as PostgreSQL
    participant MA as Merchant Application
    
    PP->>RM: Incoming Webhook
    Note over RM: Validate Source IP<br/>Verify Signatures
    RM->>RR: Forward to Webhook Route
    RR->>RC: Process Webhook
    
    RC->>DB: Retrieve Related Payment
    DB-->>RC: Payment Details
    
    RC->>RC: Transform Webhook Data
    Note over RC: Normalize to internal format
    
    RC->>DB: Update Payment Status
    
    alt Merchant Notification Required
        RC->>MA: Forward Event to Merchant
        MA-->>RC: Acknowledgement
    end
    
    RC-->>RR: Webhook Processing Result
    RR-->>RM: Format Response
    RM-->>PP: Webhook Response (200 OK)
```

## Scheduler Task Execution

The following diagram shows how the Scheduler component interacts with other components:

```mermaid
sequenceDiagram
    participant P as Scheduler (Producer)
    participant R as Redis Queue
    participant C as Scheduler (Consumer)
    participant RC as Router Core
    participant DB as PostgreSQL
    
    P->>DB: Query for Scheduled Tasks
    DB-->>P: Pending Tasks
    
    loop For each batch of tasks
        P->>R: Enqueue Task Batch
    end
    
    C->>R: Poll for Tasks
    R-->>C: Next Batch of Tasks
    
    loop For each task
        C->>RC: Execute Task Action
        RC->>DB: Update State
        RC-->>C: Action Result
        
        alt Task Failed
            C->>R: Requeue (with backoff)
        else Task Succeeded
            C->>DB: Mark Task Complete
        end
    end
```

## Key Component Responsibilities

### Router Components
- **Middleware**: Handles cross-cutting concerns like authentication, logging, and request validation
- **Routes**: Defines API endpoints and routes requests to appropriate core components
- **Core**: Contains the business logic for payment processing, refunds, webhooks, etc.
- **Connector**: Interfaces with external payment processors, handling the specific requirements of each

### Data Stores
- **PostgreSQL**: Persistent storage for all transaction data
- **Redis**: Used for caching configuration, rate limiting, and as a task queue
- **Locker**: Secure storage for sensitive payment data (PCI/PII)

### Scheduler Components
- **Producer**: Identifies tasks that need to be executed and places them in the Redis queue
- **Consumer**: Retrieves tasks from the queue and executes them

## Important Interaction Patterns

1. **Request Validation & Authentication**: All incoming requests pass through middleware layers for validation and authentication before reaching business logic.

2. **Data Transformation**: The system handles data transformation at connector boundaries, converting between internal formats and connector-specific formats.

3. **Secure Storage**: Sensitive data is tokenized through the Locker component before storage or transmission.

4. **Asynchronous Processing**: Many operations use asynchronous processing patterns, especially for long-running operations.

5. **Stateful Transactions**: The system maintains transaction state in the database, allowing for status tracking and eventual consistency.

## See Also
- [Payment Flows Documentation](../crates/router/flows/payment_flows.md)
- [Refund Flows Documentation](../crates/router/flows/refund_flows.md)
- [Webhook Flows Documentation](../crates/router/flows/webhook_flows.md)
