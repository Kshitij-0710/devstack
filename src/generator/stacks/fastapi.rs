use crate::generator::shared::{executable_file, python_dockerfile, text_file};
use crate::project::{FileEntry, Project};

pub fn files(project: &Project) -> Vec<FileEntry> {
    vec![
        text_file("docker/Dockerfile", python_dockerfile()),
        executable_file("docker/start.sh", start_script()),
        executable_file("docker/worker.sh", worker_script()),
        executable_file("docker/beat.sh", beat_script()),
        text_file("app/requirements.txt", requirements()),
        text_file("app/app/config.py", config_py(project)),
        text_file("app/app/worker.py", worker_py()),
        text_file("app/app/main.py", main_py(project)),
    ]
}

fn start_script() -> String {
    r#"#!/bin/sh
set -e
uvicorn app.main:app --host 0.0.0.0 --port ${APP_PORT:-8000} --reload
"#
    .to_string()
}

fn worker_script() -> String {
    r#"#!/bin/sh
set -e
celery -A app.worker.celery_app worker --loglevel=info
"#
    .to_string()
}

fn beat_script() -> String {
    r#"#!/bin/sh
set -e
celery -A app.worker.celery_app beat --loglevel=info
"#
    .to_string()
}

fn requirements() -> String {
    r#"celery[redis]==5.4.0
fastapi==0.115.5
gunicorn==22.0.0
psycopg[binary]==3.2.1
redis==5.0.7
uvicorn[standard]==0.32.0
"#
    .to_string()
}

fn config_py(project: &Project) -> String {
    format!(
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
        project.stack.database_name(&project.slug)
    )
}

fn worker_py() -> String {
    r#"from celery import Celery
from .config import CELERY_BROKER_URL, CELERY_RESULT_BACKEND

celery_app = Celery("app", broker=CELERY_BROKER_URL, backend=CELERY_RESULT_BACKEND)

@celery_app.task(name="tasks.ping")
def ping(payload):
    return {"received": payload}
"#
    .to_string()
}

fn main_py(project: &Project) -> String {
    format!(
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
    )
}
