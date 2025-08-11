# challenge-batch-proxy

A minimal Axum-based HTTP service scaffold with health endpoints and environment-driven configuration.

## Features
- Axum 0.7 server
- Health and readiness endpoints: `/health`, `/ready`
- BIND_HOST environment variable (defaults to `0.0.0.0:8080`)
- Structured logs via tracing with env-filter
- Graceful shutdown on Ctrl+C and SIGTERM (unix)

## Run

```sh
# optional: override bind address
BIND_HOST="127.0.0.1:8080" cargo run
```

Then open:
- http://127.0.0.1:8080/health
- http://127.0.0.1:8080/ready

## Next steps
- Add domain routes under `routes.rs`
- Introduce state (DB pools, clients) via `Router::with_state`
- Expand readiness check to verify dependencies
