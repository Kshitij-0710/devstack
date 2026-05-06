use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand, ValueEnum};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(
    name = "devstack",
    version,
    about = "One-command local dev environment generator"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Init {
        #[arg(value_enum)]
        stack: Stack,
        name: Option<String>,
        #[arg(long)]
        dir: Option<PathBuf>,
        #[arg(long)]
        force: bool,
    },
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum Stack {
    Node,
    Django,
    Flask,
    Fastapi,
}

#[derive(Clone)]
struct Project {
    stack: Stack,
    name: String,
    slug: String,
    dir: PathBuf,
}

struct FileEntry {
    path: &'static str,
    contents: String,
    executable: bool,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init {
            stack,
            name,
            dir,
            force,
        } => init(stack, name, dir, force),
    }
}

fn init(stack: Stack, name: Option<String>, dir: Option<PathBuf>, force: bool) -> Result<()> {
    let cwd = env::current_dir().context("failed to determine current directory")?;
    let project_name = name.unwrap_or_else(|| default_project_name(stack));
    let slug = sanitize_name(&project_name);

    if slug.is_empty() {
        bail!("project name must include at least one letter or number");
    }

    let target_dir = match dir {
        Some(path) => {
            if path.is_absolute() {
                path
            } else {
                cwd.join(path)
            }
        }
        None => cwd.join(&slug),
    };

    ensure_target_dir(&target_dir, force)?;

    let project = Project {
        stack,
        name: project_name,
        slug,
        dir: target_dir,
    };

    fs::create_dir_all(&project.dir)
        .with_context(|| format!("failed to create {}", project.dir.display()))?;

    for entry in files_for(&project) {
        write_entry(&project.dir, &entry)?;
    }

    println!("Created {} at {}", project.slug, project.dir.display());
    println!("Next steps:");
    println!("  cd {}", project.dir.display());
    println!("  docker compose up --build");
    println!("  visit http://localhost");

    Ok(())
}

fn ensure_target_dir(path: &Path, force: bool) -> Result<()> {
    if !path.exists() {
        return Ok(());
    }

    let mut entries =
        fs::read_dir(path).with_context(|| format!("failed to read {}", path.display()))?;

    if entries.next().is_none() || force {
        return Ok(());
    }

    bail!(
        "target directory {} already exists and is not empty; use --force to continue",
        path.display()
    )
}

fn write_entry(root: &Path, entry: &FileEntry) -> Result<()> {
    let full_path = root.join(entry.path);

    if let Some(parent) = full_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }

    fs::write(&full_path, &entry.contents)
        .with_context(|| format!("failed to write {}", full_path.display()))?;

    #[cfg(unix)]
    if entry.executable {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = fs::metadata(&full_path)?.permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&full_path, permissions)?;
    }

    Ok(())
}

fn files_for(project: &Project) -> Vec<FileEntry> {
    let mut files = vec![
        FileEntry {
            path: "README.md",
            contents: root_readme(project),
            executable: false,
        },
        FileEntry {
            path: ".env",
            contents: env_file(project),
            executable: false,
        },
        FileEntry {
            path: ".env.example",
            contents: env_file(project),
            executable: false,
        },
        FileEntry {
            path: ".gitignore",
            contents: gitignore(project),
            executable: false,
        },
        FileEntry {
            path: ".dockerignore",
            contents: dockerignore(project),
            executable: false,
        },
        FileEntry {
            path: "Makefile",
            contents: makefile(project),
            executable: false,
        },
        FileEntry {
            path: "compose.yaml",
            contents: compose_file(project),
            executable: false,
        },
        FileEntry {
            path: ".github/workflows/ci.yml",
            contents: ci_workflow(project),
            executable: false,
        },
        FileEntry {
            path: "nginx/default.conf",
            contents: nginx_conf(project),
            executable: false,
        },
    ];

    match project.stack {
        Stack::Node => files.extend(node_files(project)),
        Stack::Django => files.extend(django_files(project)),
        Stack::Flask => files.extend(flask_files(project)),
        Stack::Fastapi => files.extend(fastapi_files(project)),
    }

    files
}

fn default_project_name(stack: Stack) -> String {
    match stack {
        Stack::Node => "node-service".to_string(),
        Stack::Django => "django-service".to_string(),
        Stack::Flask => "flask-service".to_string(),
        Stack::Fastapi => "fastapi-service".to_string(),
    }
}

fn sanitize_name(name: &str) -> String {
    let mut slug = String::new();
    let mut previous_dash = false;

    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch.to_ascii_lowercase());
            previous_dash = false;
        } else if !previous_dash {
            slug.push('-');
            previous_dash = true;
        }
    }

    slug.trim_matches('-').to_string()
}

fn title_case(name: &str) -> String {
    name.split(['-', '_', ' '])
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => {
                    let mut out = String::new();
                    out.push(first.to_ascii_uppercase());
                    out.push_str(chars.as_str());
                    out
                }
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn stack_label(stack: Stack) -> &'static str {
    match stack {
        Stack::Node => "Node.js",
        Stack::Django => "Django",
        Stack::Flask => "Flask",
        Stack::Fastapi => "FastAPI",
    }
}

fn app_port(stack: Stack) -> u16 {
    match stack {
        Stack::Node => 3000,
        Stack::Django | Stack::Flask | Stack::Fastapi => 8000,
    }
}

fn root_readme(project: &Project) -> String {
    let stack = stack_label(project.stack);
    let name = title_case(&project.name);
    let extra = match project.stack {
        Stack::Node => {
            "The generated app includes Express, BullMQ, Postgres wiring, Redis, an nginx edge, and a worker service."
        }
        Stack::Django => {
            "The generated app includes Django, Postgres, Redis, Celery worker and beat processes, and an nginx edge."
        }
        Stack::Flask => {
            "The generated app includes Flask, Postgres, Redis, Celery worker and beat processes, and an nginx edge."
        }
        Stack::Fastapi => {
            "The generated app includes FastAPI, Postgres, Redis, Celery worker and beat processes, and an nginx edge."
        }
    };

    format!(
        r#"# {name}

Generated by DevStack for {stack}.

{extra}

## Quickstart

```bash
docker compose up --build
```

The application is available at `http://localhost`.

## Included

- Dockerized app runtime
- Postgres with persistent storage
- Redis for caching and jobs
- Background worker service
- GitHub Actions CI
- `.env` and `.env.example`
- Opinionated project layout for shipping fast

## Handy commands

```bash
make up
make down
make logs
make app-shell
make db-shell
```

## Layout

```text
app/
docker/
nginx/
.github/workflows/
compose.yaml
Makefile
```
"#
    )
}

fn env_file(project: &Project) -> String {
    format!(
        r#"APP_NAME={}
APP_ENV=development
APP_PORT={}
POSTGRES_DB={}
POSTGRES_USER=devstack
POSTGRES_PASSWORD=devstack
POSTGRES_HOST=postgres
POSTGRES_PORT=5432
REDIS_URL=redis://redis:6379/0
SECRET_KEY=change-me-now
ALLOWED_HOSTS=localhost,127.0.0.1
CORS_ORIGINS=http://localhost,http://127.0.0.1
CELERY_BROKER_URL=redis://redis:6379/0
CELERY_RESULT_BACKEND=redis://redis:6379/1
"#,
        project.slug,
        app_port(project.stack),
        project.slug.replace('-', "_")
    )
}

fn gitignore(project: &Project) -> String {
    match project.stack {
        Stack::Node => r#"node_modules
.env
.DS_Store
dist
coverage
"#
        .to_string(),
        Stack::Django | Stack::Flask | Stack::Fastapi => r#"__pycache__/
*.pyc
.pytest_cache/
.venv/
.env
.DS_Store
"#
        .to_string(),
    }
}

fn dockerignore(project: &Project) -> String {
    match project.stack {
        Stack::Node => r#"node_modules
npm-debug.log
.git
"#
        .to_string(),
        Stack::Django | Stack::Flask | Stack::Fastapi => r#"__pycache__/
*.pyc
.venv/
.git
"#
        .to_string(),
    }
}

fn makefile(project: &Project) -> String {
    let app_shell = match project.stack {
        Stack::Node => "docker compose exec app sh",
        Stack::Django | Stack::Flask | Stack::Fastapi => "docker compose exec app sh",
    };

    format!(
        r#"up:
	docker compose up --build

down:
	docker compose down

logs:
	docker compose logs -f

rebuild:
	docker compose build --no-cache

app-shell:
	{}

db-shell:
	docker compose exec postgres sh -lc 'psql -U "$$POSTGRES_USER" -d "$$POSTGRES_DB"'
"#,
        app_shell
    )
}

fn compose_file(project: &Project) -> String {
    let port = app_port(project.stack);
    let worker_service = match project.stack {
        Stack::Node => r#"
  worker:
    build:
      context: .
      dockerfile: docker/Dockerfile
    command: sh /workspace/docker/worker.sh
    env_file:
      - .env
    depends_on:
      postgres:
        condition: service_healthy
      redis:
        condition: service_healthy
    volumes:
      - ./app:/workspace/app
      - ./docker:/workspace/docker
      - node_modules:/workspace/app/node_modules
"#
        .to_string(),
        Stack::Django | Stack::Flask | Stack::Fastapi => r#"
  worker:
    build:
      context: .
      dockerfile: docker/Dockerfile
    command: sh /workspace/docker/worker.sh
    env_file:
      - .env
    depends_on:
      postgres:
        condition: service_healthy
      redis:
        condition: service_healthy
    volumes:
      - ./app:/workspace/app
      - ./docker:/workspace/docker

  beat:
    build:
      context: .
      dockerfile: docker/Dockerfile
    command: sh /workspace/docker/beat.sh
    env_file:
      - .env
    depends_on:
      postgres:
        condition: service_healthy
      redis:
        condition: service_healthy
    volumes:
      - ./app:/workspace/app
      - ./docker:/workspace/docker
"#
        .to_string(),
    };

    let app_volume = match project.stack {
        Stack::Node => "      - node_modules:/workspace/app/node_modules\n",
        Stack::Django | Stack::Flask | Stack::Fastapi => "",
    };

    let volumes = match project.stack {
        Stack::Node => {
            r#"
volumes:
  postgres_data:
  redis_data:
  node_modules:
"#
        }
        Stack::Django | Stack::Flask | Stack::Fastapi => {
            r#"
volumes:
  postgres_data:
  redis_data:
"#
        }
    };

    format!(
        r#"services:
  app:
    build:
      context: .
      dockerfile: docker/Dockerfile
    command: sh /workspace/docker/start.sh
    env_file:
      - .env
    depends_on:
      postgres:
        condition: service_healthy
      redis:
        condition: service_healthy
    volumes:
      - ./app:/workspace/app
      - ./docker:/workspace/docker
{app_volume}    expose:
      - "{port}"

{worker_service}  nginx:
    image: nginx:1.27-alpine
    depends_on:
      - app
    ports:
      - "80:80"
    volumes:
      - ./nginx/default.conf:/etc/nginx/conf.d/default.conf:ro

  postgres:
    image: postgres:16-alpine
    env_file:
      - .env
    environment:
      POSTGRES_DB: ${{POSTGRES_DB}}
      POSTGRES_USER: ${{POSTGRES_USER}}
      POSTGRES_PASSWORD: ${{POSTGRES_PASSWORD}}
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U $${{POSTGRES_USER}} -d $${{POSTGRES_DB}}"]
      interval: 5s
      timeout: 5s
      retries: 20
    ports:
      - "5432:5432"
    volumes:
      - postgres_data:/var/lib/postgresql/data

  redis:
    image: redis:7-alpine
    healthcheck:
      test: ["CMD", "redis-cli", "ping"]
      interval: 5s
      timeout: 3s
      retries: 20
    ports:
      - "6379:6379"
    volumes:
      - redis_data:/data
{volumes}"#
    )
}

fn ci_workflow(project: &Project) -> String {
    match project.stack {
        Stack::Node => r#"name: ci

on:
  push:
  pull_request:

jobs:
  verify:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with:
          node-version: "22"
      - name: Install dependencies
        working-directory: app
        run: npm install
      - name: Syntax check
        working-directory: app
        run: |
          node --check src/index.js
          node --check src/worker.js
      - name: Validate Compose
        run: docker compose config
      - name: Build containers
        run: docker compose build
"#
        .to_string(),
        Stack::Django | Stack::Flask | Stack::Fastapi => r#"name: ci

on:
  push:
  pull_request:

jobs:
  verify:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-python@v5
        with:
          python-version: "3.12"
      - name: Install dependencies
        run: |
          python -m pip install --upgrade pip
          pip install -r app/requirements.txt
      - name: Compile source
        run: python -m compileall app
      - name: Validate Compose
        run: docker compose config
      - name: Build containers
        run: docker compose build
"#
        .to_string(),
    }
}

fn nginx_conf(project: &Project) -> String {
    format!(
        r#"server {{
    listen 80;
    server_name _;
    client_max_body_size 32m;

    location / {{
        proxy_pass http://app:{};
        proxy_http_version 1.1;
        proxy_set_header Host $$host;
        proxy_set_header X-Real-IP $$remote_addr;
        proxy_set_header X-Forwarded-For $$proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $$scheme;
        proxy_set_header Upgrade $$http_upgrade;
        proxy_set_header Connection "upgrade";
    }}
}}
"#,
        app_port(project.stack)
    )
}

fn node_files(project: &Project) -> Vec<FileEntry> {
    vec![
        FileEntry {
            path: "docker/Dockerfile",
            contents: r#"FROM node:22-alpine
WORKDIR /workspace/app
COPY app/package*.json ./
RUN npm install
CMD ["sh", "/workspace/docker/start.sh"]
"#
            .to_string(),
            executable: false,
        },
        FileEntry {
            path: "docker/start.sh",
            contents: r#"#!/bin/sh
set -e
npm install
npm run dev
"#
            .to_string(),
            executable: true,
        },
        FileEntry {
            path: "docker/worker.sh",
            contents: r#"#!/bin/sh
set -e
npm install
npm run worker
"#
            .to_string(),
            executable: true,
        },
        FileEntry {
            path: "app/package.json",
            contents: node_package_json(project),
            executable: false,
        },
        FileEntry {
            path: "app/src/index.js",
            contents: node_index_js(project),
            executable: false,
        },
        FileEntry {
            path: "app/src/queue.js",
            contents: node_queue_js(),
            executable: false,
        },
        FileEntry {
            path: "app/src/worker.js",
            contents: node_worker_js(),
            executable: false,
        },
    ]
}

fn node_package_json(project: &Project) -> String {
    format!(
        r#"{{
  "name": "{}",
  "version": "0.1.0",
  "private": true,
  "type": "module",
  "scripts": {{
    "dev": "node src/index.js",
    "worker": "node src/worker.js"
  }},
  "dependencies": {{
    "bullmq": "^5.12.9",
    "dotenv": "^16.4.5",
    "express": "^4.19.2",
    "ioredis": "^5.4.1",
    "pg": "^8.12.0"
  }}
}}
"#,
        project.slug
    )
}

fn node_index_js(project: &Project) -> String {
    format!(
        r#"import "dotenv/config";
import express from "express";
import {{ Pool }} from "pg";
import Redis from "ioredis";
import {{ jobs }} from "./queue.js";

const app = express();
const port = Number(process.env.APP_PORT || 3000);
const pool = new Pool({{
  host: process.env.POSTGRES_HOST,
  port: Number(process.env.POSTGRES_PORT || 5432),
  database: process.env.POSTGRES_DB,
  user: process.env.POSTGRES_USER,
  password: process.env.POSTGRES_PASSWORD
}});
const redis = new Redis(process.env.REDIS_URL);

app.use(express.json());

app.get("/healthz", (_req, res) => {{
  res.json({{ status: "ok", stack: "node", service: "{}" }});
}});

app.get("/readyz", async (_req, res) => {{
  try {{
    await pool.query("select 1");
    await redis.ping();
    res.json({{ status: "ready" }});
  }} catch (error) {{
    res.status(503).json({{ status: "error", message: error.message }});
  }}
}});

app.post("/jobs/ping", async (_req, res) => {{
  const job = await jobs.add("ping", {{ service: "{}", at: new Date().toISOString() }});
  res.status(202).json({{ queued: true, id: job.id }});
}});

app.listen(port, "0.0.0.0", () => {{
  console.log(`{} listening on ${{port}}`);
}});
"#,
        project.slug, project.slug, project.slug
    )
}

fn node_queue_js() -> String {
    r#"import "dotenv/config";
import Redis from "ioredis";
import { Queue } from "bullmq";

const connection = new Redis(process.env.REDIS_URL, {
  maxRetriesPerRequest: null
});

export const jobs = new Queue("jobs", { connection });
"#
    .to_string()
}

fn node_worker_js() -> String {
    r#"import "dotenv/config";
import Redis from "ioredis";
import { Worker } from "bullmq";

const connection = new Redis(process.env.REDIS_URL, {
  maxRetriesPerRequest: null
});

new Worker(
  "jobs",
  async job => {
    console.log(`processed ${job.name}`, job.data);
  },
  { connection }
);

console.log("worker running");
"#
    .to_string()
}

fn django_files(project: &Project) -> Vec<FileEntry> {
    let slug_underscored = project.slug.replace('-', "_");
    vec![
        FileEntry {
            path: "docker/Dockerfile",
            contents: python_dockerfile(),
            executable: false,
        },
        FileEntry {
            path: "docker/start.sh",
            contents: r#"#!/bin/sh
set -e
python manage.py migrate
python manage.py runserver 0.0.0.0:${APP_PORT:-8000}
"#
            .to_string(),
            executable: true,
        },
        FileEntry {
            path: "docker/worker.sh",
            contents: r#"#!/bin/sh
set -e
celery -A config worker --loglevel=info
"#
            .to_string(),
            executable: true,
        },
        FileEntry {
            path: "docker/beat.sh",
            contents: r#"#!/bin/sh
set -e
celery -A config beat --loglevel=info
"#
            .to_string(),
            executable: true,
        },
        FileEntry {
            path: "app/requirements.txt",
            contents: r#"Django==5.2.1
celery[redis]==5.4.0
gunicorn==22.0.0
psycopg[binary]==3.2.1
redis==5.0.7
"#
            .to_string(),
            executable: false,
        },
        FileEntry {
            path: "app/manage.py",
            contents: r#"#!/usr/bin/env python
import os
import sys

def main():
    os.environ.setdefault("DJANGO_SETTINGS_MODULE", "config.settings")
    from django.core.management import execute_from_command_line
    execute_from_command_line(sys.argv)

if __name__ == "__main__":
    main()
"#
            .to_string(),
            executable: true,
        },
        FileEntry {
            path: "app/config/__init__.py",
            contents: r#"from .celery import app as celery_app

__all__ = ("celery_app",)
"#
            .to_string(),
            executable: false,
        },
        FileEntry {
            path: "app/config/settings.py",
            contents: format!(
                r#"import os
from pathlib import Path

BASE_DIR = Path(__file__).resolve().parent.parent
SECRET_KEY = os.getenv("SECRET_KEY", "change-me-now")
DEBUG = os.getenv("APP_ENV", "development") != "production"
ALLOWED_HOSTS = [host.strip() for host in os.getenv("ALLOWED_HOSTS", "localhost,127.0.0.1").split(",") if host.strip()]
INSTALLED_APPS = [
    "django.contrib.admin",
    "django.contrib.auth",
    "django.contrib.contenttypes",
    "django.contrib.sessions",
    "django.contrib.messages",
    "django.contrib.staticfiles",
    "core",
]
MIDDLEWARE = [
    "django.middleware.security.SecurityMiddleware",
    "django.contrib.sessions.middleware.SessionMiddleware",
    "django.middleware.common.CommonMiddleware",
    "django.middleware.csrf.CsrfViewMiddleware",
    "django.contrib.auth.middleware.AuthenticationMiddleware",
    "django.contrib.messages.middleware.MessageMiddleware",
    "django.middleware.clickjacking.XFrameOptionsMiddleware",
]
ROOT_URLCONF = "config.urls"
TEMPLATES = [
    {{
        "BACKEND": "django.template.backends.django.DjangoTemplates",
        "DIRS": [],
        "APP_DIRS": True,
        "OPTIONS": {{
            "context_processors": [
                "django.template.context_processors.request",
                "django.contrib.auth.context_processors.auth",
                "django.contrib.messages.context_processors.messages",
            ],
        }},
    }}
]
WSGI_APPLICATION = "config.wsgi.application"
ASGI_APPLICATION = "config.asgi.application"
DATABASES = {{
    "default": {{
        "ENGINE": "django.db.backends.postgresql",
        "NAME": os.getenv("POSTGRES_DB"),
        "USER": os.getenv("POSTGRES_USER"),
        "PASSWORD": os.getenv("POSTGRES_PASSWORD"),
        "HOST": os.getenv("POSTGRES_HOST", "postgres"),
        "PORT": os.getenv("POSTGRES_PORT", "5432"),
    }}
}}
LANGUAGE_CODE = "en-us"
TIME_ZONE = "UTC"
USE_I18N = True
USE_TZ = True
STATIC_URL = "/static/"
STATIC_ROOT = BASE_DIR / "staticfiles"
DEFAULT_AUTO_FIELD = "django.db.models.BigAutoField"
CELERY_BROKER_URL = os.getenv("CELERY_BROKER_URL", "redis://redis:6379/0")
CELERY_RESULT_BACKEND = os.getenv("CELERY_RESULT_BACKEND", "redis://redis:6379/1")
APP_NAME = "{}"
"#,
                slug_underscored
            ),
            executable: false,
        },
        FileEntry {
            path: "app/config/urls.py",
            contents: r#"from django.contrib import admin
from django.urls import include, path

urlpatterns = [
    path("admin/", admin.site.urls),
    path("", include("core.urls")),
]
"#
            .to_string(),
            executable: false,
        },
        FileEntry {
            path: "app/config/wsgi.py",
            contents: r#"import os
from django.core.wsgi import get_wsgi_application

os.environ.setdefault("DJANGO_SETTINGS_MODULE", "config.settings")
application = get_wsgi_application()
"#
            .to_string(),
            executable: false,
        },
        FileEntry {
            path: "app/config/asgi.py",
            contents: r#"import os
from django.core.asgi import get_asgi_application

os.environ.setdefault("DJANGO_SETTINGS_MODULE", "config.settings")
application = get_asgi_application()
"#
            .to_string(),
            executable: false,
        },
        FileEntry {
            path: "app/config/celery.py",
            contents: r#"import os
from celery import Celery

os.environ.setdefault("DJANGO_SETTINGS_MODULE", "config.settings")
app = Celery("config")
app.config_from_object("django.conf:settings", namespace="CELERY")
app.autodiscover_tasks()
"#
            .to_string(),
            executable: false,
        },
        FileEntry {
            path: "app/core/__init__.py",
            contents: String::new(),
            executable: false,
        },
        FileEntry {
            path: "app/core/apps.py",
            contents: r#"from django.apps import AppConfig

class CoreConfig(AppConfig):
    default_auto_field = "django.db.models.BigAutoField"
    name = "core"
"#
            .to_string(),
            executable: false,
        },
        FileEntry {
            path: "app/core/urls.py",
            contents: r#"from django.urls import path
from .views import healthz, home, ping_job, readyz

urlpatterns = [
    path("", home),
    path("healthz", healthz),
    path("readyz", readyz),
    path("jobs/ping", ping_job),
]
"#
            .to_string(),
            executable: false,
        },
        FileEntry {
            path: "app/core/views.py",
            contents: format!(
                r#"import json
import os
from django.db import connection
from django.http import HttpResponse, JsonResponse
from django.views.decorators.csrf import csrf_exempt
from redis import Redis
from .tasks import ping

def home(_request):
    return HttpResponse("DevStack {} is live")

def healthz(_request):
    return JsonResponse({{"status": "ok", "stack": "django"}})

def readyz(_request):
    try:
        with connection.cursor() as cursor:
            cursor.execute("select 1")
            cursor.fetchone()
        Redis.from_url(os.getenv("REDIS_URL", "redis://redis:6379/0")).ping()
        return JsonResponse({{"status": "ready"}})
    except Exception as error:
        return JsonResponse({{"status": "error", "message": str(error)}}, status=503)

@csrf_exempt
def ping_job(request):
    payload = {{}}
    if request.body:
        payload = json.loads(request.body.decode("utf-8"))
    task = ping.delay(payload or {{"service": "{}"}})
    return JsonResponse({{"queued": True, "task_id": task.id}}, status=202)
"#,
                project.slug, project.slug
            ),
            executable: false,
        },
        FileEntry {
            path: "app/core/tasks.py",
            contents: r#"from celery import shared_task

@shared_task
def ping(payload):
    return {"received": payload}
"#
            .to_string(),
            executable: false,
        },
    ]
}

fn flask_files(project: &Project) -> Vec<FileEntry> {
    vec![
        FileEntry {
            path: "docker/Dockerfile",
            contents: python_dockerfile(),
            executable: false,
        },
        FileEntry {
            path: "docker/start.sh",
            contents: r#"#!/bin/sh
set -e
gunicorn --bind 0.0.0.0:${APP_PORT:-8000} "app.wsgi:app"
"#
            .to_string(),
            executable: true,
        },
        FileEntry {
            path: "docker/worker.sh",
            contents: r#"#!/bin/sh
set -e
celery -A app.celery_app.celery_app worker --loglevel=info
"#
            .to_string(),
            executable: true,
        },
        FileEntry {
            path: "docker/beat.sh",
            contents: r#"#!/bin/sh
set -e
celery -A app.celery_app.celery_app beat --loglevel=info
"#
            .to_string(),
            executable: true,
        },
        FileEntry {
            path: "app/requirements.txt",
            contents: r#"celery[redis]==5.4.0
Flask==3.0.3
gunicorn==22.0.0
psycopg[binary]==3.2.1
redis==5.0.7
SQLAlchemy==2.0.36
"#
            .to_string(),
            executable: false,
        },
        FileEntry {
            path: "app/app/__init__.py",
            contents: format!(
                r#"from flask import Flask
from .routes import register_routes

def create_app():
    app = Flask("{}")
    app.config.from_object("app.config.Config")
    register_routes(app)
    return app
"#,
                project.slug
            ),
            executable: false,
        },
        FileEntry {
            path: "app/app/config.py",
            contents: r#"import os

class Config:
    APP_NAME = os.getenv("APP_NAME", "flask-service")
    POSTGRES_HOST = os.getenv("POSTGRES_HOST", "postgres")
    POSTGRES_PORT = int(os.getenv("POSTGRES_PORT", "5432"))
    POSTGRES_DB = os.getenv("POSTGRES_DB", "flask_service")
    POSTGRES_USER = os.getenv("POSTGRES_USER", "devstack")
    POSTGRES_PASSWORD = os.getenv("POSTGRES_PASSWORD", "devstack")
    REDIS_URL = os.getenv("REDIS_URL", "redis://redis:6379/0")
    CELERY_BROKER_URL = os.getenv("CELERY_BROKER_URL", REDIS_URL)
    CELERY_RESULT_BACKEND = os.getenv("CELERY_RESULT_BACKEND", "redis://redis:6379/1")
"#
            .to_string(),
            executable: false,
        },
        FileEntry {
            path: "app/app/routes.py",
            contents: format!(
                r#"from flask import jsonify, request
from psycopg import connect
from redis import Redis
from .tasks import ping

def register_routes(app):
    @app.get("/")
    def home():
        return "{} is live"

    @app.get("/healthz")
    def healthz():
        return jsonify({{"status": "ok", "stack": "flask"}})

    @app.get("/readyz")
    def readyz():
        try:
            with connect(
                host=app.config["POSTGRES_HOST"],
                port=app.config["POSTGRES_PORT"],
                dbname=app.config["POSTGRES_DB"],
                user=app.config["POSTGRES_USER"],
                password=app.config["POSTGRES_PASSWORD"],
            ) as conn:
                with conn.cursor() as cursor:
                    cursor.execute("select 1")
                    cursor.fetchone()
            Redis.from_url(app.config["REDIS_URL"]).ping()
            return jsonify({{"status": "ready"}})
        except Exception as error:
            return jsonify({{"status": "error", "message": str(error)}}), 503

    @app.post("/jobs/ping")
    def queue_ping():
        task = ping.delay(request.get_json(silent=True) or {{"service": "{}"}})
        return jsonify({{"queued": True, "task_id": task.id}}), 202
"#,
                project.slug, project.slug
            ),
            executable: false,
        },
        FileEntry {
            path: "app/app/tasks.py",
            contents: r#"from .celery_app import celery_app

@celery_app.task(name="tasks.ping")
def ping(payload):
    return {"received": payload}
"#
            .to_string(),
            executable: false,
        },
        FileEntry {
            path: "app/app/celery_app.py",
            contents: r#"from celery import Celery
from .config import Config

celery_app = Celery(
    "app",
    broker=Config.CELERY_BROKER_URL,
    backend=Config.CELERY_RESULT_BACKEND,
)
celery_app.conf.update(task_track_started=True)
"#
            .to_string(),
            executable: false,
        },
        FileEntry {
            path: "app/app/wsgi.py",
            contents: r#"from . import create_app

app = create_app()
"#
            .to_string(),
            executable: false,
        },
    ]
}

fn fastapi_files(project: &Project) -> Vec<FileEntry> {
    vec![
        FileEntry {
            path: "docker/Dockerfile",
            contents: python_dockerfile(),
            executable: false,
        },
        FileEntry {
            path: "docker/start.sh",
            contents: r#"#!/bin/sh
set -e
uvicorn app.main:app --host 0.0.0.0 --port ${APP_PORT:-8000} --reload
"#
            .to_string(),
            executable: true,
        },
        FileEntry {
            path: "docker/worker.sh",
            contents: r#"#!/bin/sh
set -e
celery -A app.worker.celery_app worker --loglevel=info
"#
            .to_string(),
            executable: true,
        },
        FileEntry {
            path: "docker/beat.sh",
            contents: r#"#!/bin/sh
set -e
celery -A app.worker.celery_app beat --loglevel=info
"#
            .to_string(),
            executable: true,
        },
        FileEntry {
            path: "app/requirements.txt",
            contents: r#"celery[redis]==5.4.0
fastapi==0.115.5
gunicorn==22.0.0
psycopg[binary]==3.2.1
redis==5.0.7
uvicorn[standard]==0.32.0
"#
            .to_string(),
            executable: false,
        },
        FileEntry {
            path: "app/app/config.py",
            contents: format!(
                r#"import os

APP_NAME = os.getenv("APP_NAME", "{}")
POSTGRES_HOST = os.getenv("POSTGRES_HOST", "postgres")
POSTGRES_PORT = int(os.getenv("POSTGRES_PORT", "5432"))
POSTGRES_DB = os.getenv("POSTGRES_DB", "{}")
POSTGRES_USER = os.getenv("POSTGRES_USER", "devstack")
POSTGRES_PASSWORD = os.getenv("POSTGRES_PASSWORD", "devstack")
REDIS_URL = os.getenv("REDIS_URL", "redis://redis:6379/0")
CELERY_BROKER_URL = os.getenv("CELERY_BROKER_URL", REDIS_URL)
CELERY_RESULT_BACKEND = os.getenv("CELERY_RESULT_BACKEND", "redis://redis:6379/1")
"#,
                project.slug,
                project.slug.replace('-', "_")
            ),
            executable: false,
        },
        FileEntry {
            path: "app/app/worker.py",
            contents: r#"from celery import Celery
from .config import CELERY_BROKER_URL, CELERY_RESULT_BACKEND

celery_app = Celery("app", broker=CELERY_BROKER_URL, backend=CELERY_RESULT_BACKEND)

@celery_app.task(name="tasks.ping")
def ping(payload):
    return {"received": payload}
"#
            .to_string(),
            executable: false,
        },
        FileEntry {
            path: "app/app/main.py",
            contents: format!(
                r#"from fastapi import FastAPI, HTTPException
from psycopg import connect
from redis import Redis
from .config import APP_NAME, POSTGRES_DB, POSTGRES_HOST, POSTGRES_PASSWORD, POSTGRES_PORT, POSTGRES_USER, REDIS_URL
from .worker import ping

app = FastAPI(title=APP_NAME)

@app.get("/")
def home():
    return {{"message": "{} is live"}}

@app.get("/healthz")
def healthz():
    return {{"status": "ok", "stack": "fastapi"}}

@app.get("/readyz")
def readyz():
    try:
        with connect(
            host=POSTGRES_HOST,
            port=POSTGRES_PORT,
            dbname=POSTGRES_DB,
            user=POSTGRES_USER,
            password=POSTGRES_PASSWORD,
        ) as conn:
            with conn.cursor() as cursor:
                cursor.execute("select 1")
                cursor.fetchone()
        Redis.from_url(REDIS_URL).ping()
        return {{"status": "ready"}}
    except Exception as error:
        raise HTTPException(status_code=503, detail=str(error))

@app.post("/jobs/ping")
def queue_ping(payload: dict | None = None):
    task = ping.delay(payload or {{"service": "{}"}})
    return {{"queued": True, "task_id": task.id}}
"#,
                project.slug, project.slug
            ),
            executable: false,
        },
    ]
}

fn python_dockerfile() -> String {
    r#"FROM python:3.12-slim
WORKDIR /workspace/app
ENV PYTHONDONTWRITEBYTECODE=1
ENV PYTHONUNBUFFERED=1
RUN apt-get update && apt-get install -y build-essential libpq-dev curl && rm -rf /var/lib/apt/lists/*
COPY app/requirements.txt /tmp/requirements.txt
RUN pip install --upgrade pip && pip install -r /tmp/requirements.txt
CMD ["sh", "/workspace/docker/start.sh"]
"#
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitizes_names() {
        assert_eq!(sanitize_name("My Fancy App"), "my-fancy-app");
        assert_eq!(sanitize_name("api__service"), "api-service");
        assert_eq!(sanitize_name("___"), "");
    }

    #[test]
    fn defaults_match_stack() {
        assert_eq!(default_project_name(Stack::Node), "node-service");
        assert_eq!(default_project_name(Stack::Django), "django-service");
        assert_eq!(app_port(Stack::Fastapi), 8000);
    }
}
