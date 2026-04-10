pub fn generate(shell: &str) -> String {
    match shell {
        "bash" => bash_init(),
        "zsh" => zsh_init(),
        "fish" => fish_init(),
        other => {
            eprintln!("warning: unsupported shell '{other}', generating bash-compatible output");
            bash_init()
        }
    }
}

fn bash_init() -> String {
    r#"# idoit shell integration (bash)
# Add to ~/.bashrc: eval "$(idoit init bash)"

__idoit_stderr_file="/tmp/.idoit-stderr-$$"

__idoit_preexec() {
    export __IDOIT_LAST_CMD="$1"
    # Wrap next command to capture stderr
}

# Use DEBUG trap to capture the command before execution
__idoit_debug_trap() {
    if [ -n "$COMP_LINE" ]; then return; fi
    if [ "$BASH_COMMAND" = "$PROMPT_COMMAND" ]; then return; fi
    export __IDOIT_LAST_CMD="$BASH_COMMAND"
}

__idoit_prompt_command() {
    export __IDOIT_LAST_EXIT=$?
}

trap '__idoit_debug_trap' DEBUG
PROMPT_COMMAND="__idoit_prompt_command${PROMPT_COMMAND:+;$PROMPT_COMMAND}"

alias ido='idoit'
alias ifix='idoit --fix'
alias ilearn='idoit --learn'
alias iexplain='idoit --explain'
"#
    .to_string()
}

fn zsh_init() -> String {
    r#"# idoit shell integration (zsh)
# Add to ~/.zshrc: eval "$(idoit init zsh)"

__idoit_preexec() {
    export __IDOIT_LAST_CMD="$1"
}

__idoit_precmd() {
    export __IDOIT_LAST_EXIT=$?
}

autoload -Uz add-zsh-hook
add-zsh-hook preexec __idoit_preexec
add-zsh-hook precmd __idoit_precmd

alias ido='idoit'
alias ifix='idoit --fix'
alias ilearn='idoit --learn'
alias iexplain='idoit --explain'
"#
    .to_string()
}

fn fish_init() -> String {
    r#"# idoit shell integration (fish)
# Add to ~/.config/fish/config.fish: idoit init fish | source

function __idoit_postexec --on-event fish_postexec
    set -gx __IDOIT_LAST_CMD $argv[1]
    set -gx __IDOIT_LAST_EXIT $status
end

alias ido 'idoit'
alias ifix 'idoit --fix'
alias ilearn 'idoit --learn'
alias iexplain 'idoit --explain'
"#
    .to_string()
}
