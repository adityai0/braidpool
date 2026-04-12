# Docker Setup

This directory contains all Docker-related files for Braidpool.

## Pre-built Images

Pre-built Docker images are automatically published to Docker Hub when changes are merged to the `dev` branch.

### Pull Images

```bash
# Braidpool Node
docker pull braidpool/braidpool:node-latest

# CPU Miner
docker pull braidpool/braidpool:miner-latest

# Bitcoin CPUNet
docker pull braidpool/braidpool:cpunet-latest
```

### Available Tags

| Image | Tags |
|-------|------|
| Node | `node-latest`, `node-dev`, `node-sha-<commit>` |
| Miner | `miner-latest`, `miner-dev`, `miner-sha-<commit>` |
| CPUNet | `cpunet-latest`, `cpunet-dev`, `cpunet-sha-<commit>` |

### Docker Hub Repository

- [braidpool/braidpool](https://hub.docker.com/r/braidpool/braidpool)

## Directory Structure

```
docker/
├── docker-compose.yml          # Development services (dashboard, api, simulator)
├── docker-compose-test.yml     # Full test environment (bitcoin + braidpool node)
├── Dockerfile.cpunet           # Bitcoin Core CPUNet build
├── node/
│   ├── Dockerfile              # Braidpool node
│   └── docker-entrypoint.sh    # Node entrypoint script
├── dashboard/
│   ├── Dockerfile              # Dashboard frontend
│   └── api/
│       └── Dockerfile          # API server
└── tests/
    └── Dockerfile              # Simulator
```

## Usage

### Development (Dashboard + API + Simulator)

```bash
cd docker
docker-compose up --build
```

Services:
- Dashboard: http://localhost:80
- API: http://localhost:5000
- Simulator: http://localhost:65433

### Full Test Environment (Bitcoin + Braidpool Node)

```bash
cd docker
docker-compose -f docker-compose-test.yml up --build
```

Services:
- Bitcoin CPUNet: ports 28332, 28333, 38332, 38338
- Braidpool Node: ports 6680, 6682, 3333

### Build Individual Images

```bash
# From repository root
docker build -t braidpool/node:latest -f docker/node/Dockerfile .
docker build -t braidpool/dashboard:latest -f docker/dashboard/Dockerfile dashboard/
docker build -t braidpool/api:latest -f docker/dashboard/api/Dockerfile dashboard/api/
```

## Stopping Services

```bash
cd docker
docker-compose down
# or for test environment
docker-compose -f docker-compose-test.yml down
```
