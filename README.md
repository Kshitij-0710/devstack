# DevStack

DevStack is a Rust CLI that generates a complete local backend development environment in one command.

Think of it like `create-next-app`, but for backend infrastructure and service foundations. Instead of spending hours wiring together Docker, Postgres, Redis, workers, nginx, environment files, and CI from scratch, you run one command and start building the actual product.

## Why DevStack exists

Starting a backend project usually means repeating the same setup work:

- Create a service skeleton
- Add Docker and Compose
- Add Postgres and Redis
- Add worker processes
- Add nginx
- Add environment files
- Add CI
- Add a sane folder layout

That work is important, but it is also repetitive. DevStack turns that setup into a single command and gives developers a base they can keep, understand, and extend.

## Current stack support

DevStack currently supports:

- `node`
- `django`
- `flask`
- `fastapi`

Each generated project includes:

- Dockerized app runtime
- `compose.yaml`
- Postgres with persistent storage
- Redis
- nginx reverse proxy
- Background worker service
- Background beat scheduler for Python stacks
- `.env` and `.env.example`
- `Makefile`
- GitHub Actions CI
- A production-minded starter layout

## Quick start

Run the generator with Cargo:

```bash
cargo run -- init django
```

You can generate any supported stack:

```bash
cargo run -- init node
cargo run -- init django
cargo run -- init flask
cargo run -- init fastapi
```

You can also provide a custom project name:

```bash
cargo run -- init fastapi my-payments-api
```

Or choose the target directory explicitly:

```bash
cargo run -- init flask --dir /tmp/my-flask-service
```

If the destination already exists and you want to allow generation there:

```bash
cargo run -- init node my-service --dir ./my-service --force
```

## CLI usage

Current command shape:

```bash
devstack init <stack> [name] [--dir <path>] [--force]
```

Arguments:

- `<stack>`: one of `node`, `django`, `flask`, or `fastapi`
- `[name]`: optional project name; if omitted, DevStack uses a stack-specific default
- `--dir <path>`: optional output directory
- `--force`: allows writing into an existing non-empty target directory

Default names:

- `node` -> `node-service`
- `django` -> `django-service`
- `flask` -> `flask-service`
- `fastapi` -> `fastapi-service`

## What gets generated

The exact generated app code differs by stack, but the overall output looks like this:

```text
your-project/
├── .env
├── .env.example
├── .dockerignore
├── .gitignore
├── Makefile
├── README.md
├── compose.yaml
├── .github/
│   └── workflows/
│       └── ci.yml
├── docker/
│   ├── Dockerfile
│   ├── start.sh
│   ├── worker.sh
│   └── beat.sh         # Python stacks only
├── nginx/
│   └── default.conf
└── app/
    └── ... stack-specific application files ...
```

### Shared infrastructure files

Every generated project includes:

- `compose.yaml`
  Brings up the app, worker processes, nginx, Postgres, and Redis.

- `.env` and `.env.example`
  Contains app name, port, database credentials, Redis URLs, and Celery settings for Python stacks.

- `Makefile`
  Convenience commands for common local tasks such as starting the stack, viewing logs, and opening shells.

- `.github/workflows/ci.yml`
  A basic CI workflow that validates the generated service and container setup.

- `nginx/default.conf`
  Reverse proxy configuration for routing traffic to the generated app container.

## Stack-specific output

### Node

The Node generator creates:

- Express HTTP service
- BullMQ-backed worker setup
- Redis queue connection
- Postgres connection wiring
- Node-specific Docker image and scripts

Generated Node app highlights:

- `app/src/index.js`
- `app/src/queue.js`
- `app/src/worker.js`
- `app/package.json`

### Django

The Django generator creates:

- Django app configuration
- Django health and readiness endpoints
- Postgres database settings
- Redis-backed Celery worker and beat setup
- Core routes and tasks

Generated Django app highlights:

- `app/manage.py`
- `app/config/settings.py`
- `app/config/urls.py`
- `app/config/celery.py`
- `app/core/views.py`
- `app/core/tasks.py`

### Flask

The Flask generator creates:

- Flask application factory
- Gunicorn startup path
- Postgres and Redis configuration
- Celery worker and beat setup
- HTTP endpoints for health, readiness, and sample job enqueueing

Generated Flask app highlights:

- `app/app/__init__.py`
- `app/app/config.py`
- `app/app/routes.py`
- `app/app/celery_app.py`
- `app/app/tasks.py`
- `app/app/wsgi.py`

### FastAPI

The FastAPI generator creates:

- FastAPI application
- Uvicorn startup path
- Postgres and Redis integration
- Celery worker and beat setup
- HTTP endpoints for health, readiness, and job enqueueing

Generated FastAPI app highlights:

- `app/app/main.py`
- `app/app/config.py`
- `app/app/worker.py`

## After generation

Once a project has been generated:

```bash
cd your-project
docker compose up --build
```

Then visit:

```text
http://localhost
```

Most generated projects also include:

- `/healthz`
- `/readyz`
- `/jobs/ping`

These endpoints make it easier to test local readiness and background job wiring immediately after setup.

## Repository structure

This repository is intentionally split so contributors can work on one concern at a time.

```text
src/
├── main.rs
├── lib.rs
├── cli.rs
├── project.rs
└── generator/
    ├── mod.rs
    ├── common.rs
    ├── shared.rs
    └── stacks/
        ├── mod.rs
        ├── node.rs
        ├── django.rs
        ├── flask.rs
        └── fastapi.rs
```

### Module responsibilities

- `src/main.rs`
  Thin binary entrypoint.

- `src/lib.rs`
  Top-level library surface that delegates to the CLI.

- `src/cli.rs`
  CLI parsing and command dispatch using `clap`.

- `src/project.rs`
  Shared domain types such as `Stack`, `Project`, and `FileEntry`.

- `src/generator/mod.rs`
  Main orchestration for project generation.

- `src/generator/common.rs`
  Shared low-level helpers for name sanitization, target directory resolution, validation, and file writing.

- `src/generator/shared.rs`
  Cross-stack templates and reusable generated files such as env files, compose config, CI config, nginx config, and shared Docker helpers.

- `src/generator/stacks/*.rs`
  One module per supported stack. Each module owns only the files and templates unique to that stack.

## Architecture principles

The codebase follows a few straightforward rules:

- Keep the binary entrypoint tiny
- Keep CLI parsing separate from generation logic
- Keep shared infrastructure templates centralized
- Keep stack-specific templates isolated per language/framework
- Prefer explicit string templates over over-abstracted code generation
- Keep adding a new stack predictable

This structure makes the project easier to review, easier to extend, and easier to contribute to without stepping on unrelated work.

## Development

### Prerequisites

To work on DevStack itself, you need:

- Rust
- Cargo

If you want to run generated projects locally, you should also have:

- Docker
- Docker Compose

### Common commands

Format the code:

```bash
cargo fmt
```

Run tests:

```bash
cargo test
```

Generate a sample project locally:

```bash
cargo run -- init django
```

Generate into a throwaway directory:

```bash
cargo run -- init node --dir /tmp/devstack-node-smoke
```

## How to add a new stack

Adding a new stack should be boring and mechanical.

1. Add the new variant to `Stack` in [src/project.rs](/Users/kshitij/development/devstack/src/project.rs:1).
2. Define stack-specific metadata such as label, default name, and port.
3. Create a new stack module in `src/generator/stacks/`.
4. Return that stack's file set from `src/generator/stacks/mod.rs`.
5. Reuse shared templates from `src/generator/shared.rs` wherever possible.
6. Keep only stack-unique files in the new stack module.
7. Add tests or smoke checks as needed.

The intended pattern is:

- Shared behavior belongs in `common.rs` or `shared.rs`
- Stack-specific output belongs in that stack's module
- Cross-cutting domain metadata belongs in `project.rs`

## Contributing guidelines

If you want to contribute, a few conventions will keep the project clean:

- Do not move stack-specific templates into shared modules unless multiple stacks truly use them
- Do not add new stacks by expanding a single giant file
- Keep generated file paths explicit and readable
- Prefer simple helpers over clever abstractions
- Preserve the current structure where contributors can reason stack by stack

Good contribution areas:

- Add new stacks
- Improve generated project quality
- Improve CI templates
- Improve Docker ergonomics
- Add better health or production defaults
- Add tests around generation output
- Add richer CLI features

## Current implementation notes

A few details about the current implementation:

- The CLI uses `clap`
- Error handling uses `anyhow`
- Generated Python stacks share a common Python Dockerfile
- Generated Node stacks use a Node-specific Dockerfile and `node_modules` volume
- Python stacks generate both a worker and a beat service
- Node currently generates a worker service but not a beat scheduler

## Example generated workflows

### Start a Django project

```bash
cargo run -- init django billing-platform
cd billing-platform
docker compose up --build
```

### Start a Node project in a custom directory

```bash
cargo run -- init node api-gateway --dir ./sandbox/api-gateway
cd ./sandbox/api-gateway
docker compose up --build
```

### Replace existing boilerplate in a prepared folder

```bash
cargo run -- init fastapi payments --dir ./services/payments --force
```

## Roadmap ideas

Some obvious future directions:

- More stacks such as Rails, Go, Laravel, NestJS, or Spring Boot
- Template versioning
- Optional flags for skipping Redis or nginx
- Optional test scaffolding
- Optional observability stack
- Better production deployment presets
- Snapshot-style tests for generated output

## Why the current structure matters

DevStack is not just about generating files. It is about making backend setup feel lightweight, understandable, and maintainable.

That means the generator itself has to be maintainable too. The codebase is structured so that:

- people can review one stack in isolation
- contributors can add features without touching everything
- shared behavior stays DRY
- generated output remains explicit rather than magical

That tradeoff is intentional.
