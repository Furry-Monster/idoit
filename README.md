# idoit

**idoit** turns natural language into shell commands. The model proposes a command; you confirm (unless you opt out) before anything runs. Use it for one-off tasks, explanations, fixing the last failed command, or a full-screen TUI workflow.

![Screenshot: idoit in use](demo.png)

## Features

- **Natural language to shell**: Describe intent in plain language; get a command aligned with your OS and shell context.
- **Safe by default**: Generated commands are shown for approval unless you use `--yes` or enable `behavior.auto_confirm` in config.
- **Multiple AI backends**: OpenAI, Anthropic, Google Gemini, DeepSeek, or local [Ollama](https://ollama.com/).
- **Shell integration**: Optional hooks record recent non-idoit commands into a local log for richer context (see `idoit init`).
- **Macros**: Save reusable prompt fragments as `@name` and expand them in prompts; definitions live beside your config.
- **Extras**: `fix` (repair last failure), `explain` (plain-language explanation of a command), `refine` (iterate on the last suggestion), `last` (re-run last idoit-generated command).

## Requirements

- **Rust toolchain** (for building from this repository): stable, recent enough for the 2021 edition.
- **Network access** to your chosen provider’s API, or a reachable **Ollama** instance for local models.
- **API keys** (cloud providers): set the environment variables your config expects, or store keys in `config.toml` if you accept that risk profile.

## Install

From a clone of this repository:

```bash
cargo install --path .
```

Ensure `~/.cargo/bin` is on your `PATH`.

## Quick start

```bash
idoit setup
idoit list files in the current directory
```

First run triggers an interactive **setup** if no config file exists yet. Then invoke **idoit** with a natural-language prompt (same as `idoit run …`).

## Commands

| Command | Purpose |
|--------|---------|
| `idoit …` | Default: treat remaining words as a natural-language prompt. |
| `idoit run …` | Same as above, explicit. |
| `idoit setup` | First-time or later reconfiguration wizard. |
| `idoit init bash` \| `zsh` \| `fish` | Print shell integration; use `eval "$(idoit init <shell>)"`. |
| `idoit config` | Show full config as TOML (default). Subcommands: `keys`, `get <key>`, `set <key> <value>`. |
| `idoit last` | Re-execute the last idoit-generated command. |
| `idoit macro NAME …` | Save a macro; reference `@NAME` in prompts. |
| `idoit tui` | Full-screen terminal UI; `-l` / `--learn` for teaching-style explanations. |
| `idoit fix` | Suggest a fix for the last failed shell command (uses history and context). |
| `idoit explain CMD …` | Explain an existing shell command in plain language. |
| `idoit refine TEXT …` | Refine the previous suggestion with extra constraints. |

Run `idoit --help` and `idoit <command> --help` for the full option list.

## Global options

These apply to subcommands that invoke the model or run commands (see `--help` for scope):

| Option | Short | Meaning |
|--------|-------|---------|
| `--learn` | `-l` | Include teaching-style explanation with the suggestion. |
| `--anyway` | `-a` | Allow proceeding when required tools may be missing (still confirms when appropriate). |
| `--dry-run` | `-d` | Print the generated command only; do not execute. |
| `--yes` | `-y` | Skip confirmation before running. |
| `--provider` | `-p` | Override provider: `openai`, `anthropic`, `gemini`, `deepseek`, `ollama`. |

## Configuration

- **File**: `~/.config/idoit/config.toml` (under `$XDG_CONFIG_HOME/idoit` when set).
- **Wizard**: `idoit setup` creates or updates settings interactively.
- **CLI**: `idoit config keys` lists dot-path keys; `idoit config get ai.provider` / `idoit config set ai.provider ollama` edit single values.

Typical environment variables for API keys (names can be changed per provider block in TOML):

| Provider | Default key env var |
|----------|---------------------|
| OpenAI | `OPENAI_API_KEY` |
| Anthropic | `ANTHROPIC_API_KEY` |
| Gemini | `GEMINI_API_KEY` |
| DeepSeek | `DEEPSEEK_API_KEY` |
| Ollama | No key; default host `http://localhost:11434` |

Relevant TOML sections include `[ai]` (provider, timeouts, temperature, models), `[behavior]` (e.g. `auto_confirm`, `learn_by_default`, `shell`, `history_path`), and `[ui]` (e.g. `color`, `verbose`, `tui_debounce_ms`). Setting `NO_COLOR` in the environment disables color output regardless of `ui.color`.

## Local data

Paths follow XDG defaults on Linux; adjust with `XDG_DATA_HOME` / `XDG_CONFIG_HOME` as usual.

| Path | Role |
|------|------|
| `~/.config/idoit/config.toml` | Main configuration. |
| `~/.config/idoit/macros.toml` | `@name` macro definitions. |
| `~/.local/share/idoit/history.json` | idoit interaction history (`last`, `refine`). |
| `~/.local/share/idoit/terminal_context.jsonl` | Recent non-idoit commands (populated when shell hooks from `idoit init` are installed). |

## Shell integration

Load the snippet for your shell so hooks can append to `terminal_context.jsonl` and support flows like `fix`:

```bash
eval "$(idoit init bash)"   # or: zsh, fish
```

Add that line to your shell rc file if you want it in every session.

## Privacy and security

Natural-language prompts, shell context, and command text may be sent to the configured AI provider (or your local Ollama). Review your organization’s policy before use. API keys are safer in the environment than in plain files; if you store them in `config.toml`, restrict file permissions.
