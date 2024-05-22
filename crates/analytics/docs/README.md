#### Starting the containers

In our use case we rely on kafka for ingesting events.
hence we can use docker compose to start all the components

```
docker compose up -d clickhouse-server kafka-ui
```

> kafka-ui is a visual tool for inspecting kafka on localhost:8090

#### Setting up Clickhouse

Once clickhouse is up & running you need to create the required tables for it

you can either visit the url (http://localhost:8123/play) in which the clickhouse-server is running to get a playground
Alternatively you can bash into the clickhouse container & execute commands manually
```
# On your local terminal
docker compose exec clickhouse-server bash

# Inside the clickhouse-server container shell
clickhouse-client --user default

# Inside the clickhouse-client shell
SHOW TABLES;
CREATE TABLE ......
```

The table creation scripts are provided [here](./scripts)

#### Running/Debugging your application
Once setup you can run your application either via docker compose or normally via cargo run

Remember to enable the kafka_events via development.toml/docker_compose.toml files

Inspect the [kafka-ui](http://localhost:8090) to check the messages being inserted in queue

If the messages/topic are available then you can run select queries on your clickhouse table to ensure data is being populated...

If the data is not being populated in clickhouse, you can check the error logs in clickhouse server via
```
# Inside the clickhouse-server container shell
tail -f /var/log/clickhouse-server/clickhouse-server.err.log
```