# Running Kafka & Clickhouse with Analytics and Events Source Configuration

This document provides instructions on how to run Kafka and Clickhouse using Docker Compose, and how to configure the analytics and events source.

## Architecture
     +------------------------+
     |       Hyperswitch      |
     +------------------------+
                |
                |
                v
     +------------------------+
     |         Kafka          |
     |  (Event Stream Broker) |
     +------------------------+
                |
                |
                v
     +------------------------+
     |  ClickHouse            |
     |  +------------------+  |
     |  | Kafka Engine     |  |
     |  |    Table         |  |
     |  +------------------+  |
     |            |           |
     |            v           |
     |  +------------------+  |
     |  | Materialized     |  |
     |  |    View (MV)     |  |
     |  +------------------+  |
     |            |           |
     |            v           |
     |  +------------------+  |
     |  | Storage Table    |  |
     |  +------------------+  |
     +------------------------+


## Starting the Containers

Docker Compose can be used to start all the components.

Run the following command:

```bash
docker compose --profile olap up -d
```
This will spawn up the following services
1. kafka
2. clickhouse
3. opensearch

## Setting up Kafka

Kafka-UI is a visual tool for inspecting Kafka and it can be accessed at `localhost:8090` to view topics, partitions, consumers & generated events.

## Setting up Clickhouse

Once Clickhouse is up and running, you can interact with it via web.

You can either visit the URL (`http://localhost:8123/play`) where the Clickhouse server is running to get a playground, or you can bash into the Clickhouse container and execute commands manually.

Run the following commands:

```bash
# On your local terminal
docker compose exec clickhouse-server bash

# Inside the clickhouse-server container shell
clickhouse-client --user default

# Inside the clickhouse-client shell
SHOW TABLES;
```

## Configuring Analytics and Events Source

To use Clickhouse and Kafka, you need to enable the `analytics.source` and update the `events.source` in the configuration file.

You can do this in either the `config/development.toml` or `config/docker_compose.toml` file.

Here's an example of how to do this:

```toml
[analytics]
source = "clickhouse"

[events]
source = "kafka"
```

After making this change, save the file and restart your application for the changes to take effect.

## Setting up Forex APIs

To use Forex services, you need to sign up and get your API keys from the following providers:

1. Primary Service 
   - Sign up for a free account and get your Primary API key [here](https://openexchangerates.org/).
   - It will be in dashboard, labeled as `app_id`.

2. Fallback Service
   - Sign up for a free account and get your Fallback API key [here](https://apilayer.com/marketplace/exchangerate_host-api).
   - It will be in dashboard, labeled as `access key`.

### Configuring Forex APIs
To enable Forex functionality, update the `config/development.toml` or `config/docker_compose.toml` file:

```toml
[analytics]
forex_enabled = true # default set to false 
```

To configure the Forex APIs, update the `config/development.toml` or `config/docker_compose.toml` file with your API keys:

```toml
[forex_api]
api_key = ""
fallback_api_key = ""
```
### Important Note
```bash
ERROR router::services::api: error: {"error":{"type":"api","message":"Failed to fetch currency exchange rate","code":"HE_00"}}
│
├─▶ Failed to fetch currency exchange rate
│
╰─▶ Could not acquire the lock for cache entry
```

_If you get the above error after setting up, simply remove the `redis` key `"{forex_cache}_lock"` by running this in shell_

```bash
redis-cli del "{forex_cache}_lock"
```

After making these changes, save the file and restart your application for the changes to take effect.

## Enabling Data Features in Dashboard

To check the data features in the dashboard, you need to enable them in the `config/dashboard.toml` configuration file.

Here's an example of how to do this:

```toml
[default.features]
audit_trail=true
system_metrics=true
global_search=true
```

## Viewing the data on OpenSearch Dashboard

To view the data on the OpenSearch dashboard perform the following steps:

- Go to the OpenSearch Dashboard home and click on `Dashboards Management` under the Management tab
- Select `Index Patterns`
- Click on `Create index pattern`
- Define an index pattern with the same name that matches your indices and click on `Next Step`
- Select a time field that will be used for time-based queries
- Save the index pattern

Now, head on to `Discover` under the `OpenSearch Dashboards` tab, to select the newly created index pattern and query the data
