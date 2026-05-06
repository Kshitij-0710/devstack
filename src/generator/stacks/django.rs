use crate::generator::shared::{executable_file, python_dockerfile, text_file};
use crate::project::{FileEntry, Project};

pub fn files(project: &Project) -> Vec<FileEntry> {
    let app_name = project.stack.database_name(&project.slug);

    vec![
        text_file("docker/Dockerfile", python_dockerfile()),
        executable_file("docker/start.sh", start_script()),
        executable_file("docker/worker.sh", worker_script()),
        executable_file("docker/beat.sh", beat_script()),
        text_file("app/requirements.txt", requirements()),
        executable_file("app/manage.py", manage_py()),
        text_file("app/config/__init__.py", config_init()),
        text_file("app/config/settings.py", settings_py(&app_name)),
        text_file("app/config/urls.py", config_urls()),
        text_file("app/config/wsgi.py", wsgi_py()),
        text_file("app/config/asgi.py", asgi_py()),
        text_file("app/config/celery.py", celery_py()),
        text_file("app/core/__init__.py", String::new()),
        text_file("app/core/apps.py", apps_py()),
        text_file("app/core/urls.py", core_urls()),
        text_file("app/core/views.py", views_py(project)),
        text_file("app/core/tasks.py", tasks_py()),
    ]
}

fn start_script() -> String {
    r#"#!/bin/sh
set -e
python manage.py migrate
python manage.py runserver 0.0.0.0:${APP_PORT:-8000}
"#
    .to_string()
}

fn worker_script() -> String {
    r#"#!/bin/sh
set -e
celery -A config worker --loglevel=info
"#
    .to_string()
}

fn beat_script() -> String {
    r#"#!/bin/sh
set -e
celery -A config beat --loglevel=info
"#
    .to_string()
}

fn requirements() -> String {
    r#"Django==5.2.1
celery[redis]==5.4.0
gunicorn==22.0.0
psycopg[binary]==3.2.1
redis==5.0.7
"#
    .to_string()
}

fn manage_py() -> String {
    r#"#!/usr/bin/env python
import os
import sys

def main():
    os.environ.setdefault("DJANGO_SETTINGS_MODULE", "config.settings")
    from django.core.management import execute_from_command_line
    execute_from_command_line(sys.argv)

if __name__ == "__main__":
    main()
"#
    .to_string()
}

fn config_init() -> String {
    r#"from .celery import app as celery_app

__all__ = ("celery_app",)
"#
    .to_string()
}

fn settings_py(app_name: &str) -> String {
    format!(
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
        app_name
    )
}

fn config_urls() -> String {
    r#"from django.contrib import admin
from django.urls import include, path

urlpatterns = [
    path("admin/", admin.site.urls),
    path("", include("core.urls")),
]
"#
    .to_string()
}

fn wsgi_py() -> String {
    r#"import os
from django.core.wsgi import get_wsgi_application

os.environ.setdefault("DJANGO_SETTINGS_MODULE", "config.settings")
application = get_wsgi_application()
"#
    .to_string()
}

fn asgi_py() -> String {
    r#"import os
from django.core.asgi import get_asgi_application

os.environ.setdefault("DJANGO_SETTINGS_MODULE", "config.settings")
application = get_asgi_application()
"#
    .to_string()
}

fn celery_py() -> String {
    r#"import os
from celery import Celery

os.environ.setdefault("DJANGO_SETTINGS_MODULE", "config.settings")
app = Celery("config")
app.config_from_object("django.conf:settings", namespace="CELERY")
app.autodiscover_tasks()
"#
    .to_string()
}

fn apps_py() -> String {
    r#"from django.apps import AppConfig

class CoreConfig(AppConfig):
    default_auto_field = "django.db.models.BigAutoField"
    name = "core"
"#
    .to_string()
}

fn core_urls() -> String {
    r#"from django.urls import path
from .views import healthz, home, ping_job, readyz

urlpatterns = [
    path("", home),
    path("healthz", healthz),
    path("readyz", readyz),
    path("jobs/ping", ping_job),
]
"#
    .to_string()
}

fn views_py(project: &Project) -> String {
    format!(
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
    )
}

fn tasks_py() -> String {
    r#"from celery import shared_task

@shared_task
def ping(payload):
    return {"received": payload}
"#
    .to_string()
}
