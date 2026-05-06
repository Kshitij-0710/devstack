# DevStack

DevStack is a Rust CLI that generates a complete local backend environment in one command.

## What it does

```bash
devstack init django
devstack init flask
devstack init fastapi
devstack init node
```

Each generated project comes with Docker, Postgres, Redis, nginx, background workers, CI, env files, and a production-minded folder layout.

## Run it

```bash
cargo run -- init django
```

You can also choose the target directory explicitly:

```bash
cargo run -- init fastapi --dir /tmp/my-api
```

## Why this feels good

- One command to get a backend foundation in place
- Supports multiple popular server stacks
- Generates files people can actually keep and ship from
- Keeps the CLI itself fast, single-binary, and easy to extend
