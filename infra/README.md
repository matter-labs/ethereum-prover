# Infra: Local Metrics Stack

This folder contains a local Prometheus + Grafana setup for `ethereum_prover`, plus a
TypeScript (Grafana Foundation SDK) dashboard generator.

## Layout

- `infra/docker-compose.yml`: Prometheus + Grafana services.
- `infra/prometheus/prometheus.yml`: Prometheus scrape config (targets host at `9898`).
- `infra/grafana/provisioning/`: Grafana datasource + dashboard provisioning.
- `infra/grafana/dashboards/`: Generated dashboard JSON output.
- `infra/dashboards/`: Dashboard source (TypeScript) and build script.

## Prerequisites

- `docker` + `docker compose`
- `node` + `yarn` (for dashboard generation)
- `ethereum_prover` running on host with metrics enabled:
  - `eth_prover_prometheus_port=9898`

## Generate dashboards

From repo root:

```sh
./infra/dashboards/build.sh
```

This writes `infra/grafana/dashboards/ethereum-prover.json`.

## Run Prometheus + Grafana

```sh
docker compose -f infra/docker-compose.yml up
```

Grafana: http://localhost:3000

Prometheus: http://localhost:9090

## Notes

- Prometheus scrapes `host.docker.internal:9898`. On Linux, `host.docker.internal`
  is provided via `extra_hosts: host-gateway` in the compose file.
