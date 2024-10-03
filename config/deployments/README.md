# Configs for deployments

## Introduction

This directory contains the configs for deployments of Hyperswitch in different hosted environments.

Hyperswitch has **3** components namely,

- router
- drainer
- scheduler
  - consumer
  - producer

We maintain configs for the `router` component for 3 different environments, namely,

- Integration Test
- Sandbox
- Production

To learn about what "router", "drainer" and "scheduler" is, please refer to the [Hyperswitch architecture][architecture] documentation.

### Tree structure

```text
config/deployments            # Root directory for the deployment configs
├── README.md                 # This file
├── drainer.toml              # Config specific to drainer
├── env_specific.toml         # Config for environment specific values which are meant to be sensitive (to be set by the user)
├── integration_test.toml     # Config specific to integration_test environment
├── production.toml           # Config specific to production environment
├── sandbox.toml              # Config specific to sandbox environment
└── scheduler                 # Directory for scheduler configs
    ├── consumer.toml         # Config specific to consumer
    └── producer.toml         # Config specific to producer
```

## Router

The `integration_test.toml`, `sandbox.toml`, and `production.toml` files are configuration files for the environments `integration_test`, `sandbox`, and `production`, respectively. These files maintain a 1:1 mapping with the environment names, and it is recommended to use the same name for the environment throughout this document.

### Generating a Config File for the Router

The `env_specific.toml` file contains values that are specific to the environment. This file is kept separate because the values in it are sensitive and are meant to be set by the user. The `env_specific.toml` file is merged with the `integration_test.toml`, `sandbox.toml`, or `production.toml` file to create the final configuration file for the router.

For example, to build and deploy Hyperswitch in the **sandbox environment**, you can duplicate the `env_specific.toml` file and rename it as `sandbox_config.toml`. Then, update the values in the file with the proper values for the sandbox environment.

The environment-specific `sandbox.toml` file, which contains the Hyperswitch recommended defaults, is merged with the `sandbox_config.toml` file to create the final configuration file called `sandbox_release.toml`. This file is marked as ready for deploying on the sandbox environment.

1. Duplicate the `env_specific.toml` file and rename it as `sandbox_config.toml`:

   ```shell
   cp config/deployments/env_specific.toml config/deployments/sandbox_config.toml
   ```

2. Update the values in the `sandbox_config.toml` file with the proper values for the sandbox environment:

   ```shell
   vi config/deployments/sandbox_config.toml
   ```

3. To merge the files you can use `cat`:

   ```shell
   cat config/deployments/sandbox.toml config/deployments/sandbox_config.toml > config/deployments/sandbox_release.toml
   ```

> [!NOTE]
> You can refer to the [`config.example.toml`][config_example] file to understand the variables that used are in the `env_specific.toml` file.

## Scheduler

The scheduler has two components, namely `consumer` and `producer`.

The `consumer.toml` and `producer.toml` files are the configuration files for the `consumer` and `producer`, respectively. These files contain the default values recommended by Hyperswitch.

### Generating a Config File for the Scheduler

Scheduler configuration files are built on top of the router configuration files. So, the `sandbox_release.toml` file is merged with the `consumer.toml` or `producer.toml` file to create the final configuration file for the scheduler.

You can use `cat` to merge the files in the terminal.

- Below is an example for consumer in sandbox environment:

  ```shell
  cat config/deployments/scheduler/consumer.toml config/deployments/sandbox_release.toml > config/deployments/consumer_sandbox_release.toml
  ```

- Below is an example for producer in sandbox environment:

  ```shell
  cat config/deployments/scheduler/producer.toml config/deployments/sandbox_release.toml > config/deployments/producer_sandbox_release.toml
  ```

## Drainer

Drainer is an independent component, and hence, the drainer configs can be used directly provided that the user updates the `drainer.toml` file with proper values before using.

## Running Hyperswitch through Docker Compose

To run the router, you can use the following snippet in the `docker-compose.yml` file:

```yaml
### Application services
hyperswitch-server:
  image: juspaydotin/hyperswitch-router:latest # This pulls the latest image from Docker Hub. If you wish to use a version without added features (like KMS), you can replace `latest` with `standalone`. However, please note that the standalone version is not recommended for production use.
  command: /local/bin/router --config-path /local/config/deployments/sandbox_release.toml # <--- Change this to the config file that is generated for the environment.
  ports:
    - "8080:8080"
  volumes:
    - ./config:/local/config
```

To run the producer, you can use the following snippet in the `docker-compose.yml` file:

```yaml
hyperswitch-producer:
  image: juspaydotin/hyperswitch-producer:latest
  command: /local/bin/scheduler --config-path /local/config/deployments/producer_sandbox_release.toml # <--- Change this to the config file that is generated for the environment.
  volumes:
    - ./config:/local/config
  environment:
    - SCHEDULER_FLOW=producer
```

To run the consumer, you can use the following snippet in the `docker-compose.yml` file:

```yaml
hyperswitch-consumer:
  image: juspaydotin/hyperswitch-consumer:latest
  command: /local/bin/scheduler --config-path /local/config/deployments/consumer_sandbox_release.toml # <--- Change this to the config file that is generated for the environment
  volumes:
    - ./config:/local/config
  environment:
    - SCHEDULER_FLOW=consumer
```

To run the drainer, you can use the following snippet in the `docker-compose.yml` file:

```yaml
hyperswitch-drainer:
  image: juspaydotin/hyperswitch-drainer:latest
  command: /local/bin/drainer --config-path /local/config/deployments/drainer.toml
  volumes:
    - ./config:/local/config
```

> [!NOTE]
> You can replace the term `sandbox` with the environment name that you are deploying to (e.g., `production`, `integration_test`, etc.) with respective changes (optional) and use the same steps to generate the final configuration file for the environment.

You can verify that the server is up and running by hitting the health check endpoint:

```shell
curl --head --request GET 'http://localhost:8080/health'
```

[architecture]: /docs/architecture.md
[config_example]: /config/config.example.toml
