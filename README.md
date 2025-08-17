# Auto-Batching Proxy for Text Embeddings

An Axum-based HTTP service for implementing auto-batching proxy for text-embeddings inference.

## Problem Statement

The idea behind this service is based on the fact that batching inference requests together in a single batch request is more efficient (especially for GPU-based inference). However, individual users might not have enough data to fill a batch.

The auto-batching proxy service should resolve this problem by automatically batching inference requests from multiple independent users, so that for users the interface looks like individual requests, but internally it is handled as a batch request.

## Features

- **Transparent Batching**: Users send individual requests, service handles batching internally
- **Configurable Parameters**: Adjustable batch size and wait time for optimal performance
- **High Performance**: Built with Rust and Axum for minimal overhead
- **Health Monitoring**: Built-in health and readiness endpoints
- **Comprehensive Metrics**: Detailed performance benchmarking included

## Quick Start

### Using Docker Compose

The easiest way to run the service is using Docker Compose, which will start both the upstream text-embeddings service and the batching proxy:

```bash
# Start both services
docker compose up -d

# Check if services are running
curl http://localhost:3000/health
curl http://localhost:3000/ready
```

### Manual Setup

1. **Start the upstream text-embeddings service:**
```bash
docker run --rm -it -p 8080:80 --pull always ghcr.io/huggingface/text-embeddings-inference:cpu-latest --model-id nomic-ai/nomic-embed-text-v1.5
```

2. **Configure the proxy service:**
```bash
cp .env.example .env
# Edit .env if needed to adjust configuration
```

3. **Run the proxy service:**
```bash
cargo run --release
```

## API Usage

### Embedding Endpoint

**Individual Request (what users send):**
```bash
curl -X POST http://localhost:3000/embed \
  -H "Content-Type: application/json" \
  -d '{"input": "What is Vector Search?"}'
```

**Response:**
```json
[0.123, -0.456, 0.789, ...]
```

### Comparison with Direct Upstream

**Direct upstream request (what proxy sends internally):**
```bash
curl -X POST http://localhost:8080/embed \
  -H "Content-Type: application/json" \
  -d '{"inputs": ["What is Vector Search?", "Hello, world!"]}'
```

**Upstream response:**
```json
[
  [0.123, -0.456, 0.789, ...],
  [0.321, -0.654, 0.987, ...]
]
```

### Health Endpoints

```bash
# Health check
curl http://localhost:3000/health

# Readiness check (includes upstream service availability)
curl http://localhost:3000/ready
```

## Configuration

The service is configured via environment variables. Key parameters include:

### Core Configuration
- `BIND_HOST` (default: `0.0.0.0:3000`): Server bind address
- `EMBEDDING_SERVICE_URL`: URL of the upstream text-embeddings service
- `RUST_LOG`: Logging configuration

### Batching Parameters

- **`EMBEDDING_BATCH_DURATION_MS`** (default: `200`): **Max Wait Time** - Maximum time in milliseconds a request will wait for other requests to accumulate into a batch
- **`EMBEDDING_BATCH_SIZE`** (default: `20`): **Max Batch Size** - Maximum number of requests that can be accumulated in a single batch
- `EMBEDDING_USE_BATCH` (default: `true`): Enable/disable batching (useful for testing)

## Benchmark Results

Implementing the batching strategy shows significant improvements in both latency and throughput. The ability to adjust batch size and wait duration allows for fine-tuning performance characteristics based on specific use cases.

Detailed benchmark reports can be found in the `benchmarks-k6` directory.

### Non-Batched Performance (Individual Requests)

| Metric | Average | Maximum | Median | Minimum | 90th Percentile | 95th Percentile |
|--------|---------|---------|--------|---------|----------------|----------------|
| http_req_duration | 21326.88 | 37069.87 | 20463.25 | 36.55 | 32851.82 | 34699.76 |
| http_req_waiting | 21326.73 | 37069.71 | 20463.18 | 36.39 | 32851.73 | 34698.90 |
| http_req_sending | 0.09 | 1.05 | 0.06 | 0.02 | 0.16 | 0.21 |
| http_req_receiving | 0.06 | 0.83 | 0.04 | 0.01 | 0.10 | 0.12 |
| http_req_blocked | 0.12 | 1.65 | 0.01 | 0.00 | 0.38 | 0.55 |
| iteration_duration | 21015.02 | 36494.84 | 20059.86 | 40.04 | 32275.70 | 34123.50 |

### Batched Performance (Auto-Batching Enabled)

| Metric | Average | Maximum | Median | Minimum | 90th Percentile | 95th Percentile |
|--------|---------|---------|--------|---------|----------------|----------------|
| http_req_duration | 8855.14 | 32164.97 | 4459.01 | 22.57 | 26199.62 | 27261.44 |
| http_req_waiting | 8855.00 | 32164.92 | 4458.77 | 22.35 | 26199.30 | 27261.35 |
| http_req_sending | 0.07 | 1.59 | 0.04 | 0.01 | 0.13 | 0.19 |
| http_req_receiving | 0.06 | 0.69 | 0.04 | 0.01 | 0.12 | 0.17 |
| http_req_blocked | 0.06 | 0.88 | 0.01 | 0.00 | 0.22 | 0.33 |
| iteration_duration | 8750.18 | 31578.02 | 4464.87 | 24.44 | 25718.59 | 26674.76 |

### Running Benchmarks

To reproduce these benchmarks:

1. **Start services:**
```bash
docker compose up -d
```

2. **Install k6:**
```bash
# On NixOS/with Nix
nix-shell -p k6

# Or via package manager
# Ubuntu: sudo snap install k6
# macOS: brew install k6
```

3. **Run benchmarks:**
```bash
# Test with batching enabled
cargo make benchmark

# Test without batching (modify .env: EMBEDDING_USE_BATCH=false)
# Restart service and run again
```

## Architecture

### Project Structure

- **`src/`** - Main application code (Axum-based service)
  - `embedding/` - Text embedding handlers and batching logic
  - `http/` - HTTP API endpoints and routing
  - `config.rs` - Configuration management
  - `main.rs` - Application entry point
- **`crates/batch/`** - Reusable batching library with generic batching logic
- **`tests-bruno/`** - Bruno API collections for manual testing and exploration
- **`benchmarks-k6/`** - K6 load testing scripts and performance reports

### How Batching Works

1. **Request Arrival**: Individual embedding requests arrive at the proxy
2. **Accumulation**: Requests are collected in a batch until either:
   - Maximum batch size is reached (`EMBEDDING_BATCH_SIZE`)
   - Maximum wait time expires (`EMBEDDING_BATCH_DURATION_MS`)
3. **Upstream Call**: Accumulated requests are sent as a single batch to the upstream service
4. **Response Distribution**: Individual responses are extracted and returned to original requesters
5. **Parallel Processing**: Multiple batches can be processed concurrently

## Development

### Prerequisites

- Rust 1.89+ (see `rust-toolchain.toml`)
- Docker (for running upstream service)
- Optional: Bruno, k6 (for testing and benchmarking)

### Quick Development Setup

#### Using Nix

For nix users you can use provided flake.nix file to set up the development environment.

To start development shell:

```sh
nix develop
```

Using `direnv` the existing `.envrc` file could be used to spin up the development environment automatically.

#### Using Cargo Make

Install `cargo make`:

```sh
cargo install cargo-make

# Show all available tasks
cargo make help

# Run the service in development mode
cargo make run

# Run tests
cargo make test

# Start upstream text-embeddings service
cargo make run-external
```

Additional tools may be required to run some tasks:
- `docker` to run text-embedding upstream
- `bruno-cli` to test api
- `k6` to run load tests

### Environment Configuration

Copy the example environment file and modify as needed:

```sh
cp .env.example .env
```

The application will automatically load environment variables from the `.env` file if it exists.
