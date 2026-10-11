use std::net::SocketAddr;
use std::path::PathBuf;

use clap::{Parser, Subcommand};
use tracing_subscriber::EnvFilter;

use alpha_drive::{client, crypto, entities, server};

#[derive(Parser, Debug)]
#[command(name = "alpha-drive", about = "Encrypted Drive authority + client helpers")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand, Debug)]
enum Cmd {
    /// Run the ciphertext block authority (HTTP). Never holds the family root key.
    Serve {
        #[arg(long, default_value = "127.0.0.1:8787")]
        listen: SocketAddr,
        #[arg(long, default_value = "./data")]
        data: PathBuf,
        /// Bearer token for API access — not a crypto key.
        #[arg(long, env = "ALPHA_DRIVE_TOKEN")]
        token: String,
    },
    /// Generate a family root capability (hex). Devices only — never the server.
    Keygen,
    /// Encrypt a local file and store it under a logical Drive path.
    Put {
        #[arg(long, default_value = "http://127.0.0.1:8787")]
        url: String,
        #[arg(long, env = "ALPHA_DRIVE_TOKEN")]
        token: String,
        #[arg(long, env = "ALPHA_DRIVE_ROOT_KEY")]
        root_key: String,
        /// e.g. files/notes.txt or projects/{id}/project.json
        path: String,
        file: PathBuf,
        #[arg(long, default_value = "blob")]
        kind: String,
    },
    /// Fetch and decrypt a logical path.
    Get {
        #[arg(long, default_value = "http://127.0.0.1:8787")]
        url: String,
        #[arg(long, env = "ALPHA_DRIVE_TOKEN")]
        token: String,
        #[arg(long, env = "ALPHA_DRIVE_ROOT_KEY")]
        root_key: String,
        path: String,
        #[arg(long)]
        out: Option<PathBuf>,
    },
    /// List encrypted root directory entries (paths + kinds).
    Ls {
        #[arg(long, default_value = "http://127.0.0.1:8787")]
        url: String,
        #[arg(long, env = "ALPHA_DRIVE_TOKEN")]
        token: String,
        #[arg(long, env = "ALPHA_DRIVE_ROOT_KEY")]
        root_key: String,
    },
    /// Write `meta/space.json` for this family Drive.
    Init {
        #[arg(long, default_value = "http://127.0.0.1:8787")]
        url: String,
        #[arg(long, env = "ALPHA_DRIVE_TOKEN")]
        token: String,
        #[arg(long, env = "ALPHA_DRIVE_ROOT_KEY")]
        root_key: String,
        #[arg(long, default_value = "Family")]
        name: String,
    },
    /// Project entity helpers (`alpha.project/1`).
    Project {
        #[command(subcommand)]
        cmd: ProjectCmd,
    },
    /// Task entity helpers (`alpha.task/2`).
    Task {
        #[command(subcommand)]
        cmd: TaskCmd,
    },
}

#[derive(Subcommand, Debug)]
enum ProjectCmd {
    Create {
        #[arg(long, default_value = "http://127.0.0.1:8787")]
        url: String,
        #[arg(long, env = "ALPHA_DRIVE_TOKEN")]
        token: String,
        #[arg(long, env = "ALPHA_DRIVE_ROOT_KEY")]
        root_key: String,
        #[arg(long)]
        title: String,
        #[arg(long)]
        slug: Option<String>,
    },
    Get {
        #[arg(long, default_value = "http://127.0.0.1:8787")]
        url: String,
        #[arg(long, env = "ALPHA_DRIVE_TOKEN")]
        token: String,
        #[arg(long, env = "ALPHA_DRIVE_ROOT_KEY")]
        root_key: String,
        id: String,
    },
    List {
        #[arg(long, default_value = "http://127.0.0.1:8787")]
        url: String,
        #[arg(long, env = "ALPHA_DRIVE_TOKEN")]
        token: String,
        #[arg(long, env = "ALPHA_DRIVE_ROOT_KEY")]
        root_key: String,
    },
}

#[derive(Subcommand, Debug)]
enum TaskCmd {
    Create {
        #[arg(long, default_value = "http://127.0.0.1:8787")]
        url: String,
        #[arg(long, env = "ALPHA_DRIVE_TOKEN")]
        token: String,
        #[arg(long, env = "ALPHA_DRIVE_ROOT_KEY")]
        root_key: String,
        /// Project id to label this task with (repeatable).
        #[arg(long = "project")]
        projects: Vec<String>,
        #[arg(long)]
        title: String,
    },
    Get {
        #[arg(long, default_value = "http://127.0.0.1:8787")]
        url: String,
        #[arg(long, env = "ALPHA_DRIVE_TOKEN")]
        token: String,
        #[arg(long, env = "ALPHA_DRIVE_ROOT_KEY")]
        root_key: String,
        id: String,
    },
    List {
        #[arg(long, default_value = "http://127.0.0.1:8787")]
        url: String,
        #[arg(long, env = "ALPHA_DRIVE_TOKEN")]
        token: String,
        #[arg(long, env = "ALPHA_DRIVE_ROOT_KEY")]
        root_key: String,
        #[arg(long)]
        project: Option<String>,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("alpha_drive=info".parse()?))
        .init();

    match Cli::parse().cmd {
        Cmd::Serve { listen, data, token } => server::serve(listen, data, token).await?,
        Cmd::Keygen => {
            let key = crypto::generate_root_key();
            println!("{}", hex::encode(key));
            eprintln!("Family root capability — store on devices only; never on the Drive server.");
            eprintln!("v0: same key on every family device. Recovery kit offline.");
        }
        Cmd::Put {
            url,
            token,
            root_key,
            path,
            file,
            kind,
        } => {
            let plaintext = std::fs::read(&file)?;
            let hash = client::put_path(&url, &token, &root_key, &path, &kind, &plaintext).await?;
            println!("{hash}");
        }
        Cmd::Get {
            url,
            token,
            root_key,
            path,
            out,
        } => {
            let plaintext = client::get_path(&url, &token, &root_key, &path).await?;
            if let Some(out) = out {
                std::fs::write(out, &plaintext)?;
            } else {
                use std::io::Write;
                std::io::stdout().write_all(&plaintext)?;
            }
        }
        Cmd::Ls {
            url,
            token,
            root_key,
        } => {
            let dir = client::list_dir(&url, &token, &root_key).await?;
            for (path, child) in dir.children {
                println!("{}\t{}", child.kind, path);
            }
        }
        Cmd::Init {
            url,
            token,
            root_key,
            name,
        } => {
            let doc = entities::init_space(&url, &token, &root_key, &name).await?;
            println!("{}", serde_json::to_string_pretty(&doc)?);
        }
        Cmd::Project { cmd } => match cmd {
            ProjectCmd::Create {
                url,
                token,
                root_key,
                title,
                slug,
            } => {
                let p =
                    entities::create_project(&url, &token, &root_key, &title, slug.as_deref())
                        .await?;
                println!("{}", serde_json::to_string_pretty(&p)?);
            }
            ProjectCmd::Get {
                url,
                token,
                root_key,
                id,
            } => {
                let p = entities::get_project(&url, &token, &root_key, &id).await?;
                println!("{}", serde_json::to_string_pretty(&p)?);
            }
            ProjectCmd::List {
                url,
                token,
                root_key,
            } => {
                let list = entities::list_projects(&url, &token, &root_key).await?;
                println!("{}", serde_json::to_string_pretty(&list)?);
            }
        },
        Cmd::Task { cmd } => match cmd {
            TaskCmd::Create {
                url,
                token,
                root_key,
                projects,
                title,
            } => {
                let t =
                    entities::create_task(&url, &token, &root_key, &projects, &title).await?;
                println!("{}", serde_json::to_string_pretty(&t)?);
            }
            TaskCmd::Get {
                url,
                token,
                root_key,
                id,
            } => {
                let t = entities::get_task(&url, &token, &root_key, &id).await?;
                println!("{}", serde_json::to_string_pretty(&t)?);
            }
            TaskCmd::List {
                url,
                token,
                root_key,
                project,
            } => {
                let list =
                    entities::list_tasks(&url, &token, &root_key, project.as_deref()).await?;
                println!("{}", serde_json::to_string_pretty(&list)?);
            }
        },
    }
    Ok(())
}
