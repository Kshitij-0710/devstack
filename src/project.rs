use clap::ValueEnum;
use std::path::PathBuf;

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum Stack {
    Node,
    Django,
    Flask,
    Fastapi,
}

#[derive(Clone, Debug)]
pub struct Project {
    pub stack: Stack,
    pub name: String,
    pub slug: String,
    pub dir: PathBuf,
}

#[derive(Clone, Debug)]
pub struct FileEntry {
    pub path: &'static str,
    pub contents: String,
    pub executable: bool,
}

impl Stack {
    pub fn default_project_name(self) -> &'static str {
        match self {
            Self::Node => "node-service",
            Self::Django => "django-service",
            Self::Flask => "flask-service",
            Self::Fastapi => "fastapi-service",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Node => "Node.js",
            Self::Django => "Django",
            Self::Flask => "Flask",
            Self::Fastapi => "FastAPI",
        }
    }

    pub fn app_port(self) -> u16 {
        match self {
            Self::Node => 3000,
            Self::Django | Self::Flask | Self::Fastapi => 8000,
        }
    }

    pub fn database_name(self, slug: &str) -> String {
        slug.replace('-', "_")
    }

    pub fn uses_beat(self) -> bool {
        !matches!(self, Self::Node)
    }

    pub fn uses_node_modules_volume(self) -> bool {
        matches!(self, Self::Node)
    }

    pub fn app_shell(self) -> &'static str {
        "docker compose exec app sh"
    }
}

impl Project {
    pub fn title_name(&self) -> String {
        title_case(&self.name)
    }
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
