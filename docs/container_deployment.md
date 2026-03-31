# Braidpool Node Container Deployment

This document describes how to run containerized Braidpool nodes using Docker and Kubernetes.

## Quick Start

### Build the Docker Image

```bash
# From repository root
docker build -t braidpool/node:latest -f docker/node/Dockerfile .
```

### Run a Single Node (Docker)

```bash
docker run -d \
  --name braidpool-node \
  -p 6680:6680 \
  -p 6682:6682 \
  -p 3333:3333 \
  -e BRAIDPOOL_NETWORK=cpunet \
  -e BRAIDPOOL_BITCOIN_HOST=host.docker.internal \
  -v braidpool-data:/data \
  braidpool/node:latest
```

### Running both the services under one 
```bash
# From repository root
cd docker
docker compose -f docker-compose-test.yml up 
```