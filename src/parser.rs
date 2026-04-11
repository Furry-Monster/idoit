use clap::{Parser, Subcommand};
use std::ffi::OsString;

use crate::config::settings::AiProviderId;

#[derive(Parser, Debug)]
#[command(
    name = "idoit",
    version,
    about = "AI-powered command line simplifier — just do it!",
    long_about = "Translate natural language into shell commands, fix mistakes, and learn as you go.\n\n\
        Routing (subcommands):\n  \
        idoit init bash|zsh|fish     shell integration script\n  \
        idoit setup                  first-time / reconfigure\n  \
        idoit config                 show settings\n  \
        idoit last                   re-run last generated command\n  \
        idoit macro NAME …           save @NAME macro\n  \
        idoit tui [-l]               full-screen TUI (--learn)\n  \
        idoit fix                    repair last failed shell command\n  \
        idoit explain 'cmd …'        explain a shell command\n  \
        idoit refine \"…\"             refine previous suggestion\n  \
        idoit run …                  explicit NL → command\n  \
        idoit …                      default: NL → command\n\n\
        Globals (any position with subcommands): --dry-run, --provider, -y, --learn, …"
)]
pub struct Args {
    #[command(flatten)]
    pub global: GlobalOpts,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

/// Options shared by all subcommands (`global = true` so they work after the verb too).
#[derive(Parser, Debug, Clone)]
pub struct GlobalOpts {
    /// Show teaching explanation alongside the generated command
    #[arg(short, long, global = true)]
    pub learn: bool,

    /// Let AI proceed even when required tools are missing (with confirmation)
    #[arg(short, long, global = true)]
    pub anyway: bool,

    /// Only print the generated command; do not execute
    #[arg(short, long, global = true)]
    pub dry_run: bool,

    /// Skip confirmation before running
    #[arg(short, long, global = true)]
    pub yes: bool,

    /// Override AI provider (openai, anthropic, gemini, ollama)
    #[arg(short, long, global = true, value_enum)]
    pub provider: Option<AiProviderId>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Print shell integration (eval "$(idoit init bash)")
    Init { shell: String },
    /// Interactive configuration wizard
    Setup,
    /// Show current configuration
    Config,
    /// Re-execute the last idoit-generated command
    Last,
    /// Save macro @NAME from the remaining words (use @NAME in prompts)
    Macro {
        name: String,
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        body: Vec<String>,
    },
    /// Full-screen TUI; use -l / --learn for teaching mode
    Tui {
        #[arg(short = 'l', long)]
        learn: bool,
    },
    /// Fix the last failed command using shell history
    Fix,
    /// Explain a shell command in plain language
    Explain {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        command: Vec<String>,
    },
    /// Refine the previous suggestion with extra constraints
    Refine {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        text: Vec<String>,
    },
    /// Explicit natural language → command (same as a bare prompt)
    Run {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        prompt: Vec<String>,
    },
    /// Natural language → command (any words not matching a built-in subcommand)
    #[command(external_subcommand)]
    Prompt(Vec<OsString>),
}

impl Args {
    pub fn join_prompt(parts: &[String]) -> String {
        parts.join(" ")
    }

    pub fn join_prompt_os(parts: &[OsString]) -> String {
        parts
            .iter()
            .map(|s| s.to_string_lossy())
            .collect::<Vec<_>>()
            .join(" ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;
    use std::ffi::OsString;

    #[test]
    fn join_prompt_joins_words() {
        assert_eq!(Args::join_prompt(&["a".into(), "b".into()]), "a b");
        assert_eq!(Args::join_prompt(&[]), "");
    }

    #[test]
    fn join_prompt_os_lossy_utf8() {
        assert_eq!(
            Args::join_prompt_os(&[OsString::from("x"), OsString::from("y")]),
            "x y"
        );
    }

    #[test]
    fn parse_init_shell() {
        let a = Args::try_parse_from(["idoit", "init", "zsh"]).unwrap();
        match a.command {
            Some(Commands::Init { shell }) => assert_eq!(shell, "zsh"),
            _ => panic!("expected Init"),
        }
    }

    #[test]
    fn parse_global_flags_after_subcommand() {
        let a = Args::try_parse_from(["idoit", "fix", "--dry-run", "-y"]).unwrap();
        assert!(a.global.dry_run);
        assert!(a.global.yes);
        assert!(matches!(a.command, Some(Commands::Fix)));
    }

    #[test]
    fn parse_prompt_external_subcommand() {
        let a = Args::try_parse_from(["idoit", "list", "files"]).unwrap();
        match a.command {
            Some(Commands::Prompt(parts)) => {
                let joined = Args::join_prompt_os(&parts);
                assert_eq!(joined, "list files");
            }
            _ => panic!("expected Prompt"),
        }
    }
}
