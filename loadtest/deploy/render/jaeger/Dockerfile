FROM docker.io/jaegertracing/jaeger:2.12.0

COPY jaeger.yaml /etc/jaeger/config.yaml
COPY entrypoint.sh /entrypoint.sh

USER root
RUN chmod +x /entrypoint.sh
USER 10001

EXPOSE 16686 4317 4318
ENTRYPOINT ["/entrypoint.sh"]
