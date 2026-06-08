# syntax=docker/dockerfile:1

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends \
  ca-certificates \
  && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY ./artifacts/polyrag /app/polyrag

RUN mkdir -p /app/uploads && chmod 777 /app/uploads
RUN groupadd -r app && useradd -r -g app app && chown -R app:app /app
USER app

EXPOSE 3000
CMD ["./polyrag"]
