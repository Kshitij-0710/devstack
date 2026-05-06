use crate::generator::shared::{executable_file, text_file};
use crate::project::{FileEntry, Project};

pub fn files(project: &Project) -> Vec<FileEntry> {
    vec![
        text_file("docker/Dockerfile", dockerfile()),
        executable_file("docker/start.sh", start_script()),
        executable_file("docker/worker.sh", worker_script()),
        text_file("app/package.json", package_json(project)),
        text_file("app/src/index.js", index_js(project)),
        text_file("app/src/queue.js", queue_js()),
        text_file("app/src/worker.js", worker_js()),
    ]
}

fn dockerfile() -> String {
    r#"FROM node:22-alpine
WORKDIR /workspace/app
COPY app/package*.json ./
RUN npm install
CMD ["sh", "/workspace/docker/start.sh"]
"#
    .to_string()
}

fn start_script() -> String {
    r#"#!/bin/sh
set -e
npm install
npm run dev
"#
    .to_string()
}

fn worker_script() -> String {
    r#"#!/bin/sh
set -e
npm install
npm run worker
"#
    .to_string()
}

fn package_json(project: &Project) -> String {
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

fn index_js(project: &Project) -> String {
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

fn queue_js() -> String {
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

fn worker_js() -> String {
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
