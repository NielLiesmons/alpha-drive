mod client;
mod crypto;
mod server;
mod store;

use std::net::SocketAddr;
use std::path::PathBuf;

use clap::{Parser, Subcommand};
use tracing_subscriber::EnvFilter;

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
    /// Generate a root capability (hex). Devices only — never the server.
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
            eprintln!("Store this root capability on devices only — never on the Drive server.");
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
    }
    Ok(())
}
