use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};

#[derive(Parser)]
#[command(name = "arktup")]
#[command(about = "arukellt toolchain manager")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    #[command(about = "Show the local toolchain state recorded in ARKTUP_HOME or ./.arktup")]
    Show,
    #[command(about = "Record a locally installed toolchain version in the metadata state")]
    Install {
        #[arg(help = "Toolchain version label to record as installed")]
        version: String,
    },
    #[command(about = "Select one previously installed version as the local default")]
    Default {
        #[arg(help = "Toolchain version label to mark as the default")]
        version: String,
    },
}

#[derive(Debug, Default, Deserialize, Serialize)]
struct ToolchainState {
    default: Option<String>,
    installed: Vec<String>,
}

fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(code) => code,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(1)
        }
    }
}

fn run(cli: Cli) -> Result<ExitCode> {
    let home = arktup_home()?;
    match cli.command {
        Command::Show => show_command(&home),
        Command::Install { version } => install_command(&home, &version),
        Command::Default { version } => default_command(&home, &version),
    }
}

fn show_command(home: &Path) -> Result<ExitCode> {
    let state = load_state(home)?;
    println!("arktup {}", env!("CARGO_PKG_VERSION"));
    println!("home: {}", home.display());
    println!("default: {}", state.default.as_deref().unwrap_or("<none>"));
    if state.installed.is_empty() {
        println!("installed: <none>");
    } else {
        println!("installed:");
        for version in state.installed {
            println!("- {version}");
        }
    }
    Ok(ExitCode::SUCCESS)
}

fn install_command(home: &Path, version: &str) -> Result<ExitCode> {
    let mut state = load_state(home)?;
    if !state.installed.iter().any(|installed| installed == version) {
        state.installed.push(version.to_owned());
        state.installed.sort();
        save_state(home, &state)?;
    }
    println!("installed toolchain: {version}");
    Ok(ExitCode::SUCCESS)
}

fn default_command(home: &Path, version: &str) -> Result<ExitCode> {
    let mut state = load_state(home)?;
    if !state.installed.iter().any(|installed| installed == version) {
        bail!("arktup: cannot set default to `{version}` because it is not installed");
    }
    state.default = Some(version.to_owned());
    save_state(home, &state)?;
    println!("default toolchain: {version}");
    Ok(ExitCode::SUCCESS)
}

fn arktup_home() -> Result<PathBuf> {
    if let Some(home) = env::var_os("ARKTUP_HOME") {
        Ok(PathBuf::from(home))
    } else {
        Ok(env::current_dir()
            .context("failed to resolve current working directory")?
            .join(".arktup"))
    }
}

fn load_state(home: &Path) -> Result<ToolchainState> {
    let path = state_path(home);
    if !path.exists() {
        return Ok(ToolchainState::default());
    }
    let source =
        fs::read_to_string(&path).with_context(|| format!("failed to read {}", path.display()))?;
    serde_json::from_str(&source).with_context(|| format!("failed to parse {}", path.display()))
}

fn save_state(home: &Path, state: &ToolchainState) -> Result<()> {
    fs::create_dir_all(home).with_context(|| format!("failed to create {}", home.display()))?;
    let path = state_path(home);
    let json = serde_json::to_string_pretty(state).context("failed to encode toolchain state")?;
    fs::write(&path, json).with_context(|| format!("failed to write {}", path.display()))
}

fn state_path(home: &Path) -> PathBuf {
    home.join("state.json")
}
