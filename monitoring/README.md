# Service Monitoring

## Components

[Promtail](https://grafana.com/docs/loki/latest/clients/promtail/#:~:text=Promtail%20is%20an%20agent%20which,Attaches%20labels%20to%20log%20streams)
: is a collector which ships the contents of local logs to a private Grafana Loki instance or Grafana Cloud. It is usually deployed to every machine that has applications needed to be monitored.

[Loki](https://grafana.com/docs/loki/latest/)
: is a search engine for logs inspired by Prometheus.

[OTEL Collector](https://opentelemetry.io/docs/collector/)
: is vendor-agnostic way to receive, process and export telemetry data.

[Tempo](https://grafana.com/docs/tempo/latest/)
: is a distributed tracing backend.

[Grafana](https://grafana.com/docs/grafana/latest/introduction/)
: is a query frontend to output data.

## How to run

```bash
cd monitoring

# start containers
docker-compose up -d

# FIXME: maybe we can remove manual setups with help of a config or automation?
```

### Set up logs monitor

1. Go to page of grafana: http://127.0.0.1:3000/
2. Use login and password "admin"
3. Add Loki data source: http://loki:3100
4. "Save & Exit" should give "Data source connected and labels found." if everything is okay.
5. Go to "Explore" tab and make a query "{job="varlogs"} |= ``".

### Set up logs monitor along with tracing

1. Navigate to [Grafana](http://localhost:3000/)
2. Enter "admin" for both username and password [skip if it asks for updating the password]
3. Add data source (Tempo)
   1. select `Tempo`
   2. set `URL` to `http://tempo:3200`
   3. save
4. Add data source (Loki)
   1. select Loki
   2. make it default
   3. set `URL` to `http://loki:3100`
   4. add `Derived fields`-
   5. set `Name` to `trace_id`
   6. set `Regex` to `trace_id":"(.*?)"(?=,|}|$)`
   7. set `URL` to `${__value.raw}`
   8. set `URL label` to `Tempo`
   9. enable `Internal link` and select `Tempo`
   10. save
5. Navigate to [Explore](http://localhost:3000/explore)
6. Add query [example, `job`=`router`]

### Notes:

- Use `trace_id` in logs to jump to Tempo view to visualize the tracing.
- Searching through recent trace ids is also possible by selecting appropriate `Service Name` in Tempo view under
  `Search` tab. The UI also provides other filter options.

## Helpful commands

`http://127.0.0.1:3100/ready`
: To get status of Loki, it should give "Ready".

`docker container ls -as`
: List running containers.

`docker exec -it monitoring_promtail_1 bash`
: Look inside of promtail container.

`docker-compose down`
: Stop containers
