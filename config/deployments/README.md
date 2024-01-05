# Configs for deployments

## Introduction

This directory contains the configs for deployments of hyperswitch in different hosted environments.

Hyperswitch has **3** components namely,

- router
  - integration_test
  - sandbox
  - production
- drainer
- scheduler
  - consumer
  - producer

To learn about what "router", "drainer" and "scheduler" is, please refer to the [Hyperswitch architecture][architecture] documentation.

### Tree structure

```tree
config/deployments      # Root directory for the deployment configs
├── README.md           # This file
├── drainer.toml        # Config specific to drainer
├── env_specific.toml   # Config for environment specific values (to be set by the user)
├── integ.toml          # Config specific to integration_test environment
├── production.toml     # Config specific to production environment
├── sandbox.toml        # Config specific to sandbox environment
└── scheduler           # Directory for scheduler configs
    ├── consumer.toml   # Config specific to consumer
    └── producer.toml   # Config specific to producer
```

## Router

The `integ.toml`, `sandbox.toml` and `production.toml` files are the configs for the different environments `integration_test`, `sandbox` and `production` respectively with the default values recommended by Hyperswitch.

### Generation of config file for Router

The `env_specific.toml` file contains the values that are specific to the environment (the user **must** update the file with proper values before it is used).

> And by `env_specific`, we mean the environment specific values (for `sandbox.toml`, you can name the `env_specific.toml` as `sandbox_config.toml` and update the values) that are expected to be set by the user **mandatorily**.
>
> From here on, we will refer to the `env_specific.toml` file as `sandbox_config.toml` file for better understanding.

To run Hyperswitch, the environment specific `sandbox.toml` file which contains the Hyperswitch recommended defaults, is merged with the `sandbox_config.toml` file to create the final config called as `sandbox_merged.toml` marking it as ready for deploying on the sandbox environment.

> Note: You can refer to the [`config.example.toml`][config_example] file to understand the variables that are in the `sandbox_config.toml` file.
>
> You can replace the the term `sandbox` with the environment name that you are deploying to (eg: `production`, `integration_test` etc.,) with respective changes (optional) and use the same steps to generate the final config for the environment.

You can use `cat` to merge the files in terminal.

```bash
# Example for sandbox environment
cat config/deployments/sandbox.toml config/deployments/sandbox_config.toml > config/deployments/sandbox_merged.toml
```

> You can replace the the term `sandbox` with the environment name that you are deploying to (eg: `production`, `integration_test` etc.,) with respective changes (optional) and use the same steps to generate the final config for the environment.

## Scheduler

The scheduler has 2 components namely, `consumer` and `producer`.

The `consumer.toml` and `producer.toml` files are the configs for the `consumer` and `producer` respectively with the default values recommended by Hyperswitch.

### Generation of config file for Scheduler

Scheduler config files are built on top of the router files. So, the `sandbox_merged.toml` file is merged with the `consumer.toml` or `producer.toml` file to create the final config for the scheduler.

You can use `cat` to merge the files in terminal.

```bash
# Example for consumer in sandbox environment
cat config/deployments/scheduler/consumer.toml config/deployments/sandbox_merged.toml > config/deployments/consumer_sandbox_merged.toml
```

```bash
# Example for producer in sandbox environment
cat config/deployments/scheduler/producer.toml config/deployments/sandbox_merged.toml > config/deployments/producer_sandbox_merged.toml
```

> You can replace the the term `sandbox` with the environment name that you are deploying to (eg: `production`, `integration_test` etc.,) with respective changes (optional) and use the same steps to generate the final config for the environment.

## Drainer

Drainer is a separate component and since it is independent, the drainer configs can be used directly given that the user updates the `drainer.toml` with proper values before using.

## Running Hyperswitch through Docker Compose

To run the router, you can use the following snippet in the `docker-compose.yml` file.

```yaml
### Application services
hyperswitch-server:
  image: juspaydotin/hyperswitch-router:latest # This pulls latest image from docker hub. If you wish to use a version without added features, you can replace `latest` with `standalone` instead but please note that the standalone version is not recommended for production use.
  command: /local/bin/router --config-path /local/config/deployments/sandbox_merged.toml # <--- Change this to the config file that is generated for the environment
  ports:
    - "8080:8080"
  volumes:
    - ./config:/local/config
```

To run the producer, you can use the following snippet in the `docker-compose.yml` file.

```yaml
hyperswitch-producer:
  image: juspaydotin/hyperswitch-producer:latest
  command: /local/bin/scheduler --config-path /local/config/deployments/producer_sandbox_merged.toml # <--- Change this to the config file that is generated for the environment
  volumes:
    - ./config:/local/config
  environment:
    - SCHEDULER_FLOW=producer
```

To run the consumer, you can use the following snippet in the `docker-compose.yml` file.

```yaml
hyperswitch-consumer:
  image: juspaydotin/hyperswitch-consumer:latest
  command: /local/bin/scheduler --config-path /local/config/deployments/consumer_sandbox_merged.toml # <--- Change this to the config file that is generated for the environment
  volumes:
    - ./config:/local/config
  environment:
    - SCHEDULER_FLOW=consumer
```

To run the drainer, you can use the following snippet in the `docker-compose.yml` file.

```yaml
hyperswitch-drainer:
  image: juspaydotin/hyperswitch-drainer:latest
  command: /local/bin/drainer --config-path /local/config/deployments/drainer.toml
  volumes:
    - ./config:/local/config
```

> You can replace the the term `sandbox` with the environment name that you are deploying to (eg: `production`, `integration_test` etc.,) with respective changes (optional) and use the same steps to generate the final config for the environment.

You can verify that the server is up and running by hitting the health check endpoint.

```bash
curl --head --request GET 'http://localhost:8080/health'
```

[architecture]: /docs/architecture.md
[config_example]: /config/config.example.toml
