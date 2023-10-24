FROM python:3.10-slim
ENV DAGSTER_HOME=/dagster
WORKDIR /dagster
RUN pip install 'dagit==1.5.3' 'dagster==1.5.3' 'sqlalchemy<2.0.0'
RUN printf 'telemetry:\n  enabled: false\n' > /dagster/dagster.yaml
ENTRYPOINT ["dagster", "dev", "-h", "0.0.0.0"]
