# docker/fluentd/Dockerfile

FROM fluent/fluentd:v1.16-debian-2
USER root
RUN ["gem",  "install", "fluent-plugin-kafka", "--no-document"]
RUN ["gem",  "install", "fluent-plugin-opensearch", "--no-document"]
USER fluent
