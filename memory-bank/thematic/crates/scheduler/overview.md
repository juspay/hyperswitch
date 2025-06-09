### **`scheduler` Crate Overview**

**Last Reviewed**: YYYY-MM-DD
**Core Maintainers**: [List maintainers if known]

#### 1. Primary Role and Scope

The `scheduler` crate is a core component of Hyperswitch responsible for managing the lifecycle of asynchronous background tasks. It functions as a generic task execution engine, employing a robust producer-consumer pattern.

*   **Producer**: This component periodically scans the primary database (specifically the `process_tracker` table) for tasks that are due for execution. These tasks are identified by their `schedule_time` and status (typically `New` or `Pending`). The producer groups these tasks into batches, updates their status to `Processing` in the database, and then enqueues these batches into a Redis stream. A distributed Redis lock ensures that only one producer instance is active at any given time, preventing duplicate task scheduling.
*   **Consumer**: This component reads task batches from the designated Redis stream using consumer groups for resilient processing. Upon fetching a batch, it updates the status of the contained tasks to `ProcessStarted` in the database. For each individual task, the consumer then invokes a specific "workflow" responsible for executing the task's business logic.
*   **Generic Engine**: The `scheduler` crate itself is designed to be task-agnostic. The actual business logic for different types of tasks (e.g., retrying a payment, syncing data, generating a report) is encapsulated within "workflow" modules. These workflows implement the `ProcessTrackerWorkflow` trait and are selected by a `ProcessTrackerWorkflows` implementation, typically provided by other crates that define the business need for the scheduled task.
*   **Distinction from `drainer`**: The `scheduler` is intended for managing stateful, often complex, background tasks that may involve multiple steps, retries, and specific lifecycle events tracked in the database. This contrasts with the `drainer` crate, which is primarily designed for high-throughput, often stateless, data transfer from Redis streams (used as write-ahead logs by services like `router`) to persistent storage like PostgreSQL.

#### 2. Task Lifecycle and Types

*   **Task Representation**: Tasks are represented as `ProcessTracker` entries in the database (`diesel_models`). Key fields include `id`, `name` (identifying the task type), `runner` (potentially specifying the workflow handler), `schedule_time`, `retry_count`, `status`, `business_status`, and `tracking_data` (for workflow-specific context).
*   **Lifecycle Stages**:
    1.  **Creation**: A task is initiated by inserting a `ProcessTracker` record into the database, typically with status `New` and a defined `schedule_time`. This can be done by any part of the Hyperswitch system needing to defer or schedule work.
    2.  **Scheduling (by Producer)**: The producer identifies due tasks (`New` or `Pending`), updates their DB status to `Processing`, and enqueues them as a batch into a Redis stream.
    3.  **Queuing**: The task batch resides in the Redis stream, awaiting consumption.
    4.  **Consumption (by Consumer)**: The consumer fetches a batch from the Redis stream. Tasks within the batch have their DB status updated to `ProcessStarted`.
    5.  **Execution**: The consumer invokes the appropriate workflow (based on task `name`/`runner`) for each task. The workflow executes the core business logic using `tracking_data`.
    6.  **Outcome (handled by the specific workflow)**:
        *   **Successful Completion**: The workflow updates the task's DB status to `Finish` and sets an appropriate `business_status` (e.g., `COMPLETED`).
        *   **Retryable Failure**: If the workflow encounters a temporary issue, it calculates a new `schedule_time` (often using retry utilities in `scheduler::utils`), increments the `retry_count`, and updates the task's DB status to `Pending`.
        *   **Non-Retryable Failure / Max Retries**: The workflow updates the task's DB status to `Finish` and sets a relevant failure `business_status` (e.g., `FAILED`).
        *   **Unhandled Workflow Error**: If a workflow execution fails catastrophically within the consumer, the consumer's generic error handler marks the task's DB status as `Finish` with a `business_status` of `GLOBAL_FAILURE`.
*   **Task Types**: As a generic engine, the `scheduler` does not define task types itself. These are determined by the `name` and `runner` fields of a `ProcessTracker` entry and are mapped to specific workflow implementations provided externally (e.g., by the `router` crate). Examples might include payment retries, refund processing, data synchronization, etc.

#### 3. Producer-Consumer Mechanism Details

*   **Database as Initial Source**: The `process_tracker` table in PostgreSQL serves as the definitive record for all tasks, their states, and scheduling parameters. Tasks are either created here directly or are rescheduled here after a retry.
*   **Redis Streams as Primary Queue**: The producer component of the scheduler reads tasks from the database and then uses Redis streams as a robust, persistent message queue to pass batches of these tasks to the consumer component.
*   **Consumer Groups**: The consumer utilizes Redis consumer groups. This allows for multiple consumer instances (if scaled horizontally, though current implementation appears single-threaded per `start_consumer` instance) to process tasks from the stream reliably, ensuring that each task batch is processed by only one consumer and providing resilience against consumer failures. Acknowledgment and deletion of messages from the stream ensure tasks are not lost or processed multiple times unintentionally.

#### 4. Interaction with `redis_interface`

The `scheduler` crate relies heavily on the `redis_interface` for:
*   **Task Queuing**: Implementing the primary message queue between its producer and consumer components using Redis Streams. This includes appending entries (task batches) to the stream and managing consumer groups for reading from the stream.
*   **Distributed Locking**: The producer uses Redis (`SETNX` with TTL) to implement a distributed lock. This ensures that across multiple instances of the Hyperswitch application (if applicable), only one producer is actively fetching tasks from the database and scheduling them, preventing race conditions and duplicate processing.
*   **Stream Operations**: Managing stream entries, including acknowledging processed messages and deleting them to prevent reprocessing.

#### 5. Interaction with `storage_impl` / `diesel_models`

The `scheduler` crate's interaction with `storage_impl` (and by extension, `diesel_models`) is fundamental for task persistence and state management:
*   **Authoritative State**: The `process_tracker` table (defined in `diesel_models` and accessed via `storage_impl` through the `ProcessTrackerInterface`) is the single source of truth for all task definitions, their current status (e.g., `New`, `Pending`, `Processing`, `ProcessStarted`, `Finish`), retry counts, scheduling times, and business-specific outcomes.
*   **Lifecycle Updates**: All significant transitions in a task's lifecycle (e.g., from `New` to `Processing` by the producer, from `Processing` to `ProcessStarted` by the consumer, and terminal states like `Finish` or rescheduled `Pending` states by workflows) are recorded in this database table.
*   **Task Fetching**: The producer queries this table to find tasks eligible for scheduling based on their `schedule_time` and `status`.

#### 6. Entry Points and Invocation

*   **Scheduler Initialization**: The scheduler's producer and consumer processes are typically started during application initialization by calling `scheduler::start_process_tracker`. This function takes the desired `SchedulerFlow` (Producer or Consumer) and a `workflow_selector` (an implementation of `ProcessTrackerWorkflows`) as arguments.
*   **Task Creation**: New tasks are introduced into the system when other services or components within Hyperswitch (e.g., the `router` crate during payment processing) insert new records into the `process_tracker` database table. This can be done directly or via the `ProcessTrackerInterface::insert_process` method.

#### 7. Error Handling and Retries

The `scheduler` provides a robust framework for error handling and task retries:
*   **Workflow-Specific Error Handling**: Individual workflows (implementing `ProcessTrackerWorkflow`) are primarily responsible for their own error handling. They can decide if an error is retryable or terminal.
*   **Retry Mechanism**: For retryable errors, workflows calculate the next execution time (often using utility functions like `scheduler::utils::get_delay`, which supports configurable backoff strategies based on retry count and predefined frequency/count pairs). They then use `ProcessTrackerInterface::retry_process` to update the task's record in the database, incrementing its `retry_count`, setting its status to `Pending`, and assigning the new `schedule_time`.
*   **Consumer-Level Error Handling**: If a workflow execution fails in an unhandled manner, the `consumer`'s generic error handler (`consumer_error_handler`) catches this. It logs the error and updates the task's status in the database to `Finish` with a `business_status` of `GLOBAL_FAILURE` to prevent it from being stuck or reprocessed indefinitely without resolution.
*   **Producer Lock**: The distributed lock prevents multiple producers from interfering, which is a form of error prevention.

#### 8. Observability

The `scheduler` crate is instrumented with comprehensive logging and metrics for monitoring and troubleshooting:
*   **Logging**: Detailed logs are emitted throughout the producer, consumer, and utility functions, providing insights into task fetching, batching, queuing, execution, and error states.
*   **Metrics**: A suite of metrics is exposed via `router_env::metrics` under the `PT_METER` (Process Tracker) global meter. Key metrics include:
    *   `TASKS_PICKED_COUNT`: Tasks fetched by the producer.
    *   `BATCHES_CREATED`: Task batches enqueued to Redis by the producer.
    *   `BATCHES_CONSUMED`: Task batches dequeued from Redis by the consumer.
    *   `TASK_CONSUMED`: Individual tasks dequeued by the consumer.
    *   `TASK_PROCESSED`: Tasks for which a workflow has completed execution (successfully or with handled error).
    *   `TASK_FINISHED`: Tasks that have reached a terminal `Finish` state in the database.
    *   `TASK_RETRIED`: Tasks that have been rescheduled for a retry.
    *   `CONSUMER_OPS` (Histogram): Provides timing information for consumer operations, such as the delay between a task's scheduled time and its actual pickup.
    *   (Note: The `PAYMENT_COUNT` metric, while present, indicates a common use-case rather than a generic scheduler function.)

#### 9. Configuration

The `scheduler`'s behavior is controlled via `SchedulerSettings`, typically loaded from application configuration files. Key configurable parameters include:
*   **General Settings**:
    *   `loop_interval`: The base interval (in milliseconds) for the main loops of the producer and consumer.
    *   `graceful_shutdown_interval`: The time (in milliseconds) the scheduler will wait for active tasks to complete during a graceful shutdown.
    *   `stream`: The name of the Redis stream used for inter-component task queuing.
*   **Producer Settings (`ProducerSettings`)**:
    *   `upper_fetch_limit` / `lower_fetch_limit`: Define the time window (in seconds, relative to the current time) used by the producer to query the database for due tasks.
    *   `lock_key`: The specific Redis key name for the distributed producer lock.
    *   `lock_ttl`: The time-to-live (in seconds) for the producer's distributed lock in Redis.
    *   `batch_size`: The maximum number of tasks to include in a single batch when writing to the Redis stream.
*   **Consumer Settings (`ConsumerSettings`)**:
    *   `disabled`: A boolean flag to completely disable the consumer component (e.g., for maintenance or specific deployment configurations).
    *   `consumer_group`: The name of the Redis consumer group that consumers will join to read tasks from the stream.

*(Note: The `DrainerSettings` found within `scheduler::settings` are likely related to a distinct drainer functionality that might be co-located or share a settings module. For clarity in the `scheduler` overview, it's best to focus on producer/consumer settings unless its direct integration with the scheduler's core task processing is evident.)*
