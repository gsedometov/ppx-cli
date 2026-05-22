# pplx

CLI utility for [Perplexity AI](https://perplexity.ai) search and deep research, written in Rust.

Supports four modes — quick search, pro, reasoning, and deep research — with cited sources returned for every query.

## Installation

```bash
cargo install --git https://github.com/gsedometov/pplx
```

Requires [Rust](https://rustup.rs/) and Cargo.

## Setup

Set your Perplexity API key:

```bash
pplx auth pplx-xxxxxxxxxxxx
```

Or run `pplx auth` for an interactive prompt.

The key is saved to `~/.pplx/config.toml`. You can also set the `PERPLEXITY_API_KEY` environment variable as a fallback.

## Usage

```bash
# Quick search (default)
pplx 'What is the USD to KZT exchange rate?'

# Specific mode
pplx -m pro 'query'
pplx -m reasoning 'query'
pplx -m deep-research 'query'
```

### Options

```bash
# System prompt
pplx -s 'Be concise, bullet points only' 'benefits of Rust'

# Recency filter: day, week, month, year
pplx -r week 'latest AI news'

# Domain filter (comma-separated)
pplx -d 'arxiv.org,nature.com' 'quantum computing advances'

# JSON output
pplx --json 'query'

# Custom tokens and temperature
pplx --max-tokens 8192 -t 0.5 'detailed explanation of X'
```

## Modes

| Flag               | Model                 | Use case                        | Timeout |
|--------------------|-----------------------|---------------------------------|---------|
| `-m search`        | `sonar`               | Quick factual search (default)  | 120s    |
| `-m pro`           | `sonar-pro`           | More thorough search            | 120s    |
| `-m reasoning`     | `sonar-reasoning`     | Reasoning-heavy queries         | 300s    |
| `-m deep-research` | `sonar-deep-research` | In-depth multi-minute research  | 600s    |

## Flags

| Flag            | Short | Default  | Description                              |
|-----------------|-------|----------|------------------------------------------|
| `--mode`        | `-m`  | search   | Mode: search, pro, reasoning, deep-research |
| `--system`      | `-s`  | —        | System prompt                            |
| `--max-tokens`  |       | 4096     | Max response tokens                      |
| `--recency`     | `-r`  | —        | Recency filter: day, week, month, year   |
| `--domains`     | `-d`  | —        | Domain allowlist (comma-separated)       |
| `--json`        |       | false    | Output raw JSON                          |
| `--temperature` | `-t`  | 0.2      | Sampling temperature (0.0–2.0)           |

## JSON Output

Use `--json` for programmatic access:

```json
{
  "choices": [{ "message": { "content": "..." } }],
  "citations": ["https://...", ...],
  "usage": {
    "prompt_tokens": 13,
    "completion_tokens": 103,
    "total_tokens": 116
  }
}
```

## License

MIT
