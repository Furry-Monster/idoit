use clap::Parser;

#[derive(Parser, Debug)]
#[command(
    name = "idoit",
    version,
    about = "AI-powered command line simplifier — just do it!",
    long_about = "Translate natural language into shell commands, fix mistakes, and learn as you go.\n\n\
        Examples:\n  \
        idoit find files containing \"TODO\" in src/\n  \
        idoit compress this folder as tar.gz\n  \
        idoit --fix\n  \
        idoit --learn git rebase"
)]
pub struct Cli {
    /// Natural language description of what you want to do
    #[arg(trailing_var_arg = true)]
    pub args: Vec<String>,

    /// Fix the last failed command
    #[arg(short, long)]
    pub fix: bool,

    /// Show a teaching explanation alongside the command
    #[arg(short, long)]
    pub learn: bool,

    /// Let AI do whatever it takes (with confirmation)
    #[arg(short, long)]
    pub anyway: bool,

    /// Only show the generated command, don't execute
    #[arg(short, long)]
    pub dry_run: bool,

    /// Skip the confirmation prompt
    #[arg(short, long)]
    pub yes: bool,

    /// Override the AI provider (openai, anthropic, ollama)
    #[arg(short, long)]
    pub provider: Option<String>,

    /// Show or edit configuration
    #[arg(long)]
    pub config: bool,
}

impl Cli {
    pub fn prompt(&self) -> String {
        self.args.join(" ")
    }

    pub fn has_prompt(&self) -> bool {
        !self.args.is_empty()
    }
}
