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
__idoit_data_dir="${XDG_DATA_HOME:-$HOME/.local/share}/idoit"
__idoit_term_log="$__idoit_data_dir/terminal_context.jsonl"

__idoit_append_terminal_session() {
    local cmd="$1"
    [ -z "$cmd" ] && return 0
    case "$cmd" in
        idoit*|ido|ifix|ilearn|iexplain) return 0 ;;
    esac
    cmd="${cmd//$'\n'/ }"
    mkdir -p "$__idoit_data_dir" 2>/dev/null || true
    printf '%s\t%s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ 2>/dev/null || date +%s)" "$cmd" >> "$__idoit_term_log" 2>/dev/null || true
}

__idoit_preexec() {
    export __IDOIT_LAST_CMD="$1"
}

__idoit_debug_trap() {
    if [ -n "$COMP_LINE" ]; then return; fi
    if [ "$BASH_COMMAND" = "$PROMPT_COMMAND" ]; then return; fi
    export __IDOIT_LAST_CMD="$BASH_COMMAND"
}

__idoit_prompt_command() {
    export __IDOIT_LAST_EXIT=$?
    if [ -n "${__IDOIT_LAST_CMD-}" ]; then
        export __IDOIT_COMPLETED_CMD="$__IDOIT_LAST_CMD"
        __idoit_append_terminal_session "$__IDOIT_LAST_CMD"
    fi
}

trap '__idoit_debug_trap' DEBUG
PROMPT_COMMAND="__idoit_prompt_command${PROMPT_COMMAND:+;$PROMPT_COMMAND}"

alias ido='idoit'
alias ifix='idoit fix'
alias ilearn='idoit --learn'
alias iexplain='idoit explain'
"#
    .to_string()
}

fn zsh_init() -> String {
    r#"# idoit shell integration (zsh)
# Add to ~/.zshrc: eval "$(idoit init zsh)"

__idoit_data_dir="${XDG_DATA_HOME:-$HOME/.local/share}/idoit"
__idoit_term_log="$__idoit_data_dir/terminal_context.jsonl"

__idoit_append_terminal_session() {
    local cmd="$1"
    [[ -z "$cmd" ]] && return 0
    case "$cmd" in
        idoit*|ido|ifix|ilearn|iexplain) return 0 ;;
    esac
    cmd="${cmd//$'\n'/ }"
    mkdir -p "$__idoit_data_dir" 2>/dev/null || true
    printf '%s\t%s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ 2>/dev/null || date +%s)" "$cmd" >> "$__idoit_term_log" 2>/dev/null || true
}

__idoit_preexec() {
    export __IDOIT_LAST_CMD="$1"
}

__idoit_precmd() {
    export __IDOIT_LAST_EXIT=$?
    if [ -n "${__IDOIT_LAST_CMD-}" ]; then
        export __IDOIT_COMPLETED_CMD="$__IDOIT_LAST_CMD"
        __idoit_append_terminal_session "$__IDOIT_LAST_CMD"
    fi
}

autoload -Uz add-zsh-hook
add-zsh-hook preexec __idoit_preexec
add-zsh-hook precmd __idoit_precmd

alias ido='idoit'
alias ifix='idoit fix'
alias ilearn='idoit --learn'
alias iexplain='idoit explain'
"#
    .to_string()
}

fn fish_init() -> String {
    r#"# idoit shell integration (fish)
# Add to ~/.config/fish/config.fish: idoit init fish | source

if set -q XDG_DATA_HOME
    set -gx __idoit_data_dir "$XDG_DATA_HOME/idoit"
else
    set -gx __idoit_data_dir "$HOME/.local/share/idoit"
end
set -gx __idoit_term_log "$__idoit_data_dir/terminal_context.jsonl"

function __idoit_append_terminal_session
    set -l cmd "$argv[1]"
    test -z "$cmd"; and return
    set -l base (string split ' ' -- $cmd)[1]
    switch $base
        case idoit ido ifix ilearn iexplain
            return
    end
    set cmd (string replace -a \n ' ' "$cmd")
    mkdir -p "$__idoit_data_dir" 2>/dev/null
    set -l ts (date -u +%Y-%m-%dT%H:%M:%SZ 2>/dev/null; or date +%s)
    printf '%s\t%s\n' "$ts" "$cmd" >> "$__idoit_term_log" 2>/dev/null
end

function __idoit_postexec --on-event fish_postexec
    set -gx __IDOIT_COMPLETED_CMD $argv[1]
    set -gx __IDOIT_LAST_CMD $argv[1]
    set -gx __IDOIT_LAST_EXIT $status
    __idoit_append_terminal_session "$argv[1]"
end

alias ido 'idoit'
alias ifix 'idoit fix'
alias ilearn 'idoit --learn'
alias iexplain 'idoit explain'
"#
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::generate;

    #[test]
    fn generate_bash_has_prompt_command_hook() {
        let s = generate("bash");
        assert!(s.contains("__idoit_prompt_command"));
        assert!(s.contains("PROMPT_COMMAND="));
        assert!(s.contains("alias ido='idoit'"));
    }

    #[test]
    fn generate_zsh_uses_zsh_hooks() {
        let s = generate("zsh");
        assert!(s.contains("add-zsh-hook"));
        assert!(s.contains("__idoit_precmd"));
    }

    #[test]
    fn generate_fish_uses_fish_postexec() {
        let s = generate("fish");
        assert!(s.contains("fish_postexec"));
        assert!(s.contains("function __idoit_append_terminal_session"));
    }

    #[test]
    fn generate_unknown_falls_back_to_bash_script() {
        let bash = generate("bash");
        let other = generate("some-unknown-shell");
        assert_eq!(bash, other);
    }
}
