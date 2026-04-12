# Docker Setup

This directory contains all Docker-related files for Braidpool.

## Pre-built Images

Pre-built Docker images are automatically published to Docker Hub when changes are merged to the `dev` branch.

### Pull Images

```bash
# Braidpool Node
docker pull braidpool/node:latest

# CPU Miner
docker pull braidpool/miner:latest

# Bitcoin CPUNet
docker pull braidpool/bitcoin-cpunet:latest
```

### Available Tags

| Tag | Description |
|-----|-------------|
| `latest` | Latest build from dev branch |
| `dev` | Alias for latest dev build |
| `sha-<commit>` | Specific commit (e.g., `sha-a1b2c3d`) |

### Docker Hub Repositories

- [braidpool/node](https://hub.docker.com/r/braidpool/node)
- [braidpool/miner](https://hub.docker.com/r/braidpool/miner)
- [braidpool/bitcoin-cpunet](https://hub.docker.com/r/braidpool/bitcoin-cpunet)

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
