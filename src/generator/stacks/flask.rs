use crate::generator::shared::{executable_file, python_dockerfile, text_file};
use crate::project::{FileEntry, Project};

pub fn files(project: &Project) -> Vec<FileEntry> {
    vec![
        text_file("docker/Dockerfile", python_dockerfile()),
        executable_file("docker/start.sh", start_script()),
        executable_file("docker/worker.sh", worker_script()),
        executable_file("docker/beat.sh", beat_script()),
        text_file("app/requirements.txt", requirements()),
        text_file("app/app/__init__.py", app_init(project)),
        text_file("app/app/config.py", config_py(project)),
        text_file("app/app/routes.py", routes_py(project)),
        text_file("app/app/tasks.py", tasks_py()),
        text_file("app/app/celery_app.py", celery_app_py()),
        text_file("app/app/wsgi.py", wsgi_py()),
    ]
}

fn start_script() -> String {
    r#"#!/bin/sh
set -e
gunicorn --bind 0.0.0.0:${APP_PORT:-8000} "app.wsgi:app"
"#
    .to_string()
}

fn worker_script() -> String {
    r#"#!/bin/sh
set -e
celery -A app.celery_app.celery_app worker --loglevel=info
"#
    .to_string()
}

fn beat_script() -> String {
    r#"#!/bin/sh
set -e
celery -A app.celery_app.celery_app beat --loglevel=info
"#
    .to_string()
}

fn requirements() -> String {
    r#"celery[redis]==5.4.0
Flask==3.0.3
gunicorn==22.0.0
psycopg[binary]==3.2.1
redis==5.0.7
SQLAlchemy==2.0.36
"#
    .to_string()
}

fn app_init(project: &Project) -> String {
    format!(
        r#"from flask import Flask
from .routes import register_routes

def create_app():
    app = Flask("{}")
    app.config.from_object("app.config.Config")
    register_routes(app)
    return app
"#,
        project.slug
    )
}

fn config_py(project: &Project) -> String {
    format!(
        r#"import os

class Config:
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

fn routes_py(project: &Project) -> String {
    format!(
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
    )
}

fn tasks_py() -> String {
    r#"from .celery_app import celery_app

@celery_app.task(name="tasks.ping")
def ping(payload):
    return {"received": payload}
"#
    .to_string()
}

fn celery_app_py() -> String {
    r#"from celery import Celery
from .config import Config

celery_app = Celery(
    "app",
    broker=Config.CELERY_BROKER_URL,
    backend=Config.CELERY_RESULT_BACKEND,
)
celery_app.conf.update(task_track_started=True)
"#
    .to_string()
}

fn wsgi_py() -> String {
    r#"from . import create_app

app = create_app()
"#
    .to_string()
}
