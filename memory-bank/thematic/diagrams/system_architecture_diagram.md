# Hyperswitch System Architecture Diagram

---
**Last Updated:** 2025-05-27  
**Documentation Status:** Complete
---

This document provides a comprehensive system architecture diagram for the Hyperswitch payment orchestration platform, showing the key components and their relationships.

## High-Level System Architecture

The following diagram illustrates the high-level architecture of the Hyperswitch system, including all major components, their relationships, and data flows:

```mermaid
graph TD
    %% External clients
    MerchantApp[Merchant Application] --> APIGateway[API Gateway]
    APIGateway --> Router
    
    %% Main components
    subgraph HyperswitchSystem["Hyperswitch System"]
        %% Router component and subcomponents
        subgraph RouterService["Router Service"]
            Router[Router API Service] --> CoreModule[Core Module]
            Router --> RoutesModule[Routes Module]
            Router --> ConnectorModule[Connector Module]
            Router --> MiddlewareModule[Middleware Module]
            Router --> ServicesModule[Services Module]
            Router --> ConfigModule[Config Module]
            
            %% Core module breakdown
            CoreModule --> PaymentsCore[Payments Core]
            CoreModule --> RefundsCore[Refunds Core]
            CoreModule --> WebhooksCore[Webhooks Core]
            CoreModule --> RoutingCore[Routing Core]
            CoreModule --> MandatesCore[Mandates Core]
            CoreModule --> AuthCore[Authentication Core]
        end
        
        %% Scheduler components
        subgraph SchedulerService["Scheduler Service"]
            Producer[Job Scheduler/Producer]
            Consumer[Job Executor/Consumer]
            Producer --> Consumer
        end
        
        %% Locker component
        Locker[Locker - Secure Storage]
        
        %% Data stores
        PostgreSQL[PostgreSQL Database]
        Redis[Redis Cache & Queue]
        
        %% Connections between components
        Router --> PostgreSQL
        Router --> Redis
        Router --> Locker
        
        Producer --> PostgreSQL
        Producer --> Redis
        Consumer --> PostgreSQL
        Consumer --> Redis
        Consumer --> Router
    end
    
    %% External payment processors
    subgraph PaymentProcessors["Payment Processors"]
        Stripe[Stripe]
        PayPal[PayPal]
        Adyen[Adyen]
        Checkout[Checkout.com]
        OtherProcessors[Other Processors...]
    end
    
    %% Monitoring stack
    subgraph MonitoringSystem["Monitoring System"]
        OTelCollector[OpenTelemetry Collector]
        Prometheus[Prometheus]
        Loki[Loki]
        Tempo[Tempo]
        Grafana[Grafana Dashboard]
        
        OTelCollector --> Prometheus
        OTelCollector --> Loki
        OTelCollector --> Tempo
        Prometheus --> Grafana
        Loki --> Grafana
        Tempo --> Grafana
    end
    
    %% Connections to external systems
    ConnectorModule --> PaymentProcessors
    Router --> OTelCollector
    SchedulerService --> OTelCollector
    
    %% Additional metadata
    classDef mainComponents fill:#f9f,stroke:#333,stroke-width:2px;
    classDef dataStores fill:#bbf,stroke:#333,stroke-width:2px;
    classDef external fill:#bfb,stroke:#333,stroke-width:1px;
    classDef monitoring fill:#fbb,stroke:#333,stroke-width:1px;
    
    class Router,CoreModule,SchedulerService,Producer,Consumer mainComponents;
    class PostgreSQL,Redis,Locker dataStores;
    class PaymentProcessors,MerchantApp,APIGateway external;
    class MonitoringSystem,OTelCollector,Prometheus,Loki,Tempo,Grafana monitoring;
```

## Architecture Components

### Client-Facing Components
- **Merchant Application**: External applications that integrate with Hyperswitch
- **API Gateway**: Entry point for all client requests

### Core Hyperswitch Components
- **Router Service**: Main application processing all payment requests
  - **Core Module**: Contains essential business logic
  - **Routes Module**: Defines API endpoints
  - **Connector Module**: Interfaces with payment processors
  - **Middleware Module**: Handles cross-cutting concerns
  - **Services Module**: Provides shared services
  - **Config Module**: Manages configuration

- **Scheduler Service**:
  - **Job Scheduler/Producer**: Schedules tasks for later execution
  - **Job Executor/Consumer**: Executes scheduled tasks

- **Locker**: Secure storage component for sensitive data

### Data Stores
- **PostgreSQL**: Primary database for persistent storage
- **Redis**: Used for caching and task queuing

### External Systems
- **Payment Processors**: Third-party payment services integrated with Hyperswitch

### Monitoring Components
- **OpenTelemetry Collector**: Collects metrics, logs, and traces
- **Prometheus**: Stores and processes metrics
- **Loki**: Stores and processes logs
- **Tempo**: Stores and processes traces
- **Grafana**: Visualizes monitoring data

## Key Design Considerations

1. **Modularity**: The system is designed with clear separation of concerns, allowing components to be developed and scaled independently.

2. **Security**: Sensitive payment data is handled through the dedicated Locker component.

3. **Reliability**: The system includes monitoring and observability tools to ensure reliable operation.

4. **Extensibility**: The Connector Module provides a standardized interface for adding new payment processors.

5. **Performance**: Redis caching and asynchronous task processing through the Scheduler optimize performance.

## See Also
- [Router Code Structure](../crates/router/architecture/code_structure.md)
- [System Patterns Documentation](../../systemPatterns.md)
