# HyperSwitch Architecture

- [Introduction](#introduction)
- [Router](#router)
- [Scheduler](#scheduler)
  - [Producer (Job scheduler)](#producer-job-scheduler)
  - [Consumer (Job executor)](#consumer-job-executor)
- [Database](#database)
  - [Postgres](#postgres)
  - [Redis](#redis)
- [Locker](#locker)
- [Monitoring](#monitoring)

## Introduction

Hyperswitch comprises two distinct app services: **Router** and **Scheduler** which in turn consists of **Producer** and **Consumer**, where each service has its specific responsibilities to process payment-related tasks efficiently.

<p align="center">
<img src="../docs/imgs/hyperswitch-architecture.png" alt="HyperSwitch Architecture" style="width:60%">
<p align="center"><b>Fig.1 - Typical Deployment</b></p>
</p>

## Router

The Router is the main component of Hyperswitch, serving as the primary crate where all the core payment functionalities are implemented. It is a crucial component responsible for managing and coordinating different aspects of the payment processing system. Within the Router, the core payment flows serve as the central hub through which all payment activities are directed. When a payment request is received, it goes through the Router, which handles important processing and routing tasks.

## Scheduler

Suppose a scenario where a customer has saved their card details in your application, but for security reasons, you want to remove the saved card information after a certain period.
To automate this process, Scheduler comes into picture. It schedules a task with a specific time for execution and stores it in the database. When the scheduled time arrives, the job associated with the task starts executing, here in this case, allowing the saved card details to be deleted automatically. One other situation in which we use this service in Hyperswitch is when we want to notify the merchant that their api key is about to expire.

### Producer (Job scheduler)

The Producer is one of the components responsible for the Scheduler's functionality. Its primary responsibility is to handle the tracking of tasks which are yet to be executed. When the Router Service inserts a new task into the database, specifying a scheduled time, the producer retrieves the task from the database when the scheduled time is up and proceeds to group or batch these tasks together. These batches of tasks are then stored in a Redis queue, ready for execution, which will be picked up by consumer service.

### Consumer (Job executor)

The Consumer is another key component of the Scheduler. Its main role is to retrieve batches of tasks from the Redis queue for processing, which were previously added by the Producer. Once the tasks are retrieved, the Consumer executes them. It ensures that the tasks within the batches are handled promptly and in accordance with the required processing logic.

## Database

### Postgres

The application relies on a PostgreSQL database for storing various types of data, including customer information, merchant details, payment-related data, and other relevant information. The application maintains a master-database and replica-database setup to optimize read and write operations.

### Redis

In addition to the database, Hyperswitch incorporates Redis for two main purposes. It is used to **cache** frequently accessed data in order to decrease the application latencies and reduce the load on the database. It is also used as a **queuing mechanism** by the Scheduler.

## Locker

The application utilizes a Rust locker built with a GDPR compliant PII (personal identifiable information) storage. It also uses secure encryption algorithms to be fully compliant with **PCI DSS** (Payment Card Industry Data Security Standard) requirements, this ensures that all payment-related data is handled and stored securely. You can find the source code of locker [here](https://github.com/juspay/hyperswitch-card-vault).

## Monitoring

<p align="center">
<img src="../docs/imgs/hyperswitch-monitoring-architecture.png" alt="HyperSwitch Monitoring Architecture" style="width:70%">
<p align="center"><b>Fig.2 - HyperSwitch Monitoring Architecture</b></p>
</p>

The monitoring services in Hyperswitch ensure the effective collection and analysis of metrics to monitor the system's performance.

Hyperswitch pushes the metrics and traces in **OTLP** format to the [OpenTelemetry collector]. [Prometheus] utilizes a pull-based model, where it periodically retrieves application metrics from the OpenTelemetry collector. [Promtail] scrapes application logs from the router, which in turn are pushed to the [Loki] instance. Users can query and visualize the logs in Grafana through Loki. [Tempo] is used for querying the application traces.

Except for the OpenTelemetry collector, all other monitoring services like Loki, Tempo, Prometheus can be easily replaced with a preferred equivalent, with minimal to no code changes.

[OpenTelemetry collector]: https://opentelemetry.io/docs/collector/
[Prometheus]: https://prometheus.io/docs/introduction/overview/
[Promtail]: https://grafana.com/docs/loki/latest/clients/promtail/
[Loki]: https://grafana.com/docs/loki/latest/
[Tempo]: https://grafana.com/docs/tempo/latest/
