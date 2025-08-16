# challenge-batch-proxy

A minimal Axum-based HTTP service scaffold with health endpoints and environment-driven configuration.

## Environment Configuration

Copy the example environment file and modify as needed:

```sh
cp .env.example .env
```

The application will automatically load environment variables from the `.env` file if it exists.

## Development

This project uses cargo-make for task automation. Install it if you haven't already:

```sh
cargo install cargo-make
```

Available tasks:

```sh
# Show all available tasks
cargo make help

# Development workflows
cargo make validate     # format + lint + test
cargo make watch        # watch and run the application

# Individual tasks
cargo make build        # build the project
cargo make test         # run tests
cargo make fmt          # format code
cargo make clippy       # run clippy lints with fixes
cargo make lint         # run linting (clippy + fmt)
cargo make run          # run the application

# CI workflow
cargo make ci           # audit + format-check + lint-check + test
```

## Run

```sh
# Using cargo-make
cargo make run

# With custom bind address (override .env)
BIND_HOST="127.0.0.1:8080" cargo make run
```

Then open:
- http://127.0.0.1:3000/health
- http://127.0.0.1:3000/ready
