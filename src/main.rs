use clap::{Parser, Subcommand, ValueEnum};
use colored::*;
use indicatif::ProgressBar;
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use std::env;
use std::io::{self, Write};
use std::path::PathBuf;
use std::time::Duration;

const API_BASE: &str = "https://api.perplexity.ai";

// ── Config ──────────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Default)]
struct Config {
    api_key: Option<String>,
}

fn config_path() -> PathBuf {
    dirs::home_dir()
        .expect("Cannot determine home directory")
        .join(".pplx")
        .join("config.toml")
}

fn load_config() -> Config {
    let path = config_path();
    if path.exists() {
        let content = std::fs::read_to_string(&path).unwrap_or_default();
        toml::from_str(&content).unwrap_or_default()
    } else {
        Config::default()
    }
}

fn save_config(config: &Config) {
    let path = config_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("Cannot create config directory");
    }
    let content = toml::to_string_pretty(config).expect("Cannot serialize config");
    std::fs::write(&path, content).expect("Cannot write config file");
}

/// Resolve API key: config file first, then env var.
fn resolve_api_key() -> Result<String, String> {
    // 1. Config file
    let config = load_config();
    if let Some(ref key) = config.api_key {
        if !key.is_empty() {
            return Ok(key.clone());
        }
    }
    // 2. Environment variable
    if let Ok(key) = env::var("PERPLEXITY_API_KEY") {
        if !key.is_empty() {
            return Ok(key);
        }
    }
    Err("No API key found. Run `pplx auth` to set one, or set PERPLEXITY_API_KEY.".into())
}

// ── CLI ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, ValueEnum)]
enum Mode {
    Search,
    Pro,
    Reasoning,
    DeepResearch,
}

impl std::fmt::Display for Mode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Mode::Search => write!(f, "sonar"),
            Mode::Pro => write!(f, "sonar-pro"),
            Mode::Reasoning => write!(f, "sonar-reasoning"),
            Mode::DeepResearch => write!(f, "sonar-deep-research"),
        }
    }
}

#[derive(Parser)]
#[command(name = "pplx")]
#[command(about = "Perplexity AI search & deep research CLI")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Search query or research topic
    query: Vec<String>,

    /// Model mode: search, pro, reasoning, deep-research
    #[arg(short, long, default_value = "search")]
    mode: Mode,

    /// System prompt to guide the response
    #[arg(short, long)]
    system: Option<String>,

    /// Max tokens in the response
    #[arg(long, default_value_t = 4096)]
    max_tokens: u32,

    /// Search recency filter: day, week, month, year
    #[arg(short, long)]
    recency: Option<String>,

    /// Return domain filter (comma-separated)
    #[arg(short, long)]
    domains: Option<String>,

    /// Output as JSON
    #[arg(long, default_value_t = false)]
    json: bool,

    /// Temperature (0.0 - 2.0)
    #[arg(short, long, default_value_t = 0.2)]
    temperature: f32,
}

#[derive(Subcommand)]
enum Commands {
    /// Configure your Perplexity API key
    Auth {
        /// API key (will prompt interactively if not provided)
        key: Option<String>,
    },
}

// ── Auth command ────────────────────────────────────────────────────────────

fn cmd_auth(key: Option<String>) {
    let api_key = match key {
        Some(k) => k,
        None => {
            print!("{}", "Enter your Perplexity API key: ".cyan());
            io::stdout().flush().unwrap();
            let mut input = String::new();
            io::stdin().read_line(&mut input).expect("Failed to read input");
            input.trim().to_string()
        }
    };

    if api_key.is_empty() {
        eprintln!("{}", "Error: API key cannot be empty.".red());
        std::process::exit(1);
    }

    let mut config = load_config();
    config.api_key = Some(api_key);
    save_config(&config);

    println!(
        "{} Saved to {}",
        "✓".green().bold(),
        config_path().display()
    );
}

// ── API types ───────────────────────────────────────────────────────────────

#[derive(Serialize)]
struct Message {
    role: String,
    content: String,
}

#[derive(Serialize)]
struct RequestBody {
    model: String,
    messages: Vec<Message>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    search_recency_filter: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    search_domain_filter: Option<Vec<String>>,
}

#[derive(Deserialize, Serialize, Debug)]
struct ResponseBody {
    choices: Vec<Choice>,
    #[serde(default)]
    citations: Vec<String>,
    usage: Usage,
}

#[derive(Deserialize, Serialize, Debug)]
struct Choice {
    message: ResponseMessage,
}

#[derive(Deserialize, Serialize, Debug)]
struct ResponseMessage {
    content: String,
}

#[derive(Deserialize, Serialize, Debug)]
struct Usage {
    prompt_tokens: u32,
    completion_tokens: u32,
    total_tokens: u32,
}

// ── Main ────────────────────────────────────────────────────────────────────

fn main() {
    let cli = Cli::parse();

    // Handle subcommands
    match &cli.command {
        Some(Commands::Auth { key }) => {
            cmd_auth(key.clone());
            return;
        }
        None => {}
    }

    if cli.query.is_empty() {
        eprintln!(
            "{}",
            "Error: provide a query. Usage: pplx \"your query here\"".red()
        );
        std::process::exit(1);
    }

    let api_key = match resolve_api_key() {
        Ok(k) => k,
        Err(msg) => {
            eprintln!("{}", msg.red());
            std::process::exit(1);
        }
    };

    let query = cli.query.join(" ");
    let model = cli.mode.to_string();

    let mut messages = Vec::new();

    if let Some(ref sys) = cli.system {
        messages.push(Message {
            role: "system".into(),
            content: sys.clone(),
        });
    }

    messages.push(Message {
        role: "user".into(),
        content: query.clone(),
    });

    let domain_filter = cli.domains.as_ref().map(|d| {
        d.split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
    });

    let body = RequestBody {
        model: model.clone(),
        messages,
        max_tokens: Some(cli.max_tokens),
        temperature: Some(cli.temperature),
        search_recency_filter: cli.recency.clone(),
        search_domain_filter: domain_filter,
    };

    let spinner_msg = match &cli.mode {
        Mode::DeepResearch => "Conducting deep research...",
        Mode::Reasoning => "Reasoning...",
        Mode::Pro => "Searching (pro)...",
        Mode::Search => "Searching...",
    };

    let pb = ProgressBar::new_spinner();
    pb.set_style(indicatif::ProgressStyle::default_spinner().tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏ "));
    pb.set_message(format!("{} {}", "⟳".cyan(), spinner_msg));

    // Deep research can take a long time (minutes)
    let timeout = match &cli.mode {
        Mode::DeepResearch => Duration::from_secs(600),
        Mode::Reasoning => Duration::from_secs(300),
        _ => Duration::from_secs(120),
    };

    let client = Client::builder().timeout(timeout).build().unwrap_or_else(|e| {
        eprintln!("{} {}", "HTTP client error:".red(), e);
        std::process::exit(1);
    });

    let url = format!("{}/chat/completions", API_BASE);

    let resp = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&body)
        .send();

    pb.finish_and_clear();

    match resp {
        Ok(response) => {
            let status = response.status();
            if !status.is_success() {
                let body_text = response.text().unwrap_or_else(|_| "(no body)".into());
                eprintln!("{} API error ({}): {}", "✗".red(), status, body_text);
                std::process::exit(1);
            }

            let parsed: Result<ResponseBody, _> = response.json();
            match parsed {
                Ok(data) => {
                    if cli.json {
                        println!("{}", serde_json::to_string_pretty(&data).unwrap());
                    } else {
                        print_response(&data, &model);
                    }
                }
                Err(e) => {
                    eprintln!("{} Failed to parse response: {}", "✗".red(), e);
                    std::process::exit(1);
                }
            }
        }
        Err(e) => {
            if e.is_timeout() {
                eprintln!(
                    "{} Request timed out. Deep research can take several minutes — try increasing timeout or simplifying the query.",
                    "✗".red()
                );
            } else {
                eprintln!("{} Request failed: {}", "✗".red(), e);
            }
            std::process::exit(1);
        }
    }
}

fn print_response(data: &ResponseBody, model: &str) {
    if let Some(choice) = data.choices.first() {
        println!();
        println!("{}", choice.message.content);
        println!();

        if !data.citations.is_empty() {
            let header = format!("Sources ({}) ", data.citations.len());
            println!("{}", header.bold().green());
            for (i, url) in data.citations.iter().enumerate() {
                println!("  {}. {}", i + 1, url.blue().underline());
            }
            println!();
        }
    }

    let usage_line = format!(
        "Model: {} | Tokens: {} prompt + {} completion = {} total",
        model.cyan(),
        data.usage.prompt_tokens,
        data.usage.completion_tokens,
        data.usage.total_tokens,
    );
    println!("{}", usage_line.dimmed());
}
