# fluentd/Dockerfile

FROM fluent/fluentd:v1.16-1
USER root
RUN ["gem", "install", "fluent-plugin-elasticsearch", "--no-document", "--version", "5.4.3"]
USER fluent
