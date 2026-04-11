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
#
# Captures the previous command's stderr into a file and exports __IDOIT_LAST_STDERR (read by `idoit fix`).
# Limits: no capture for pipelines, subshells (BASH_SUBSHELL!=0), or commands skipped below; child shells
# of an outer command do not contribute to the parent's stderr file.

__idoit_data_dir="${XDG_DATA_HOME:-$HOME/.local/share}/idoit"
__idoit_term_log="$__idoit_data_dir/terminal_context.jsonl"
__idoit_stderr_file="$__idoit_data_dir/last-stderr-$$.txt"
__idoit_in_command=0

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

__idoit_debug_trap() {
    if [ -n "$COMP_LINE" ]; then return; fi
    if [ "$BASH_COMMAND" = "$PROMPT_COMMAND" ]; then return; fi
    export __IDOIT_LAST_CMD="$BASH_COMMAND"
    case "$BASH_COMMAND" in
        idoit*|ido|ifix|ilearn|iexplain*) return ;;
    esac
    if [ "${BASH_SUBSHELL:-0}" -ne 0 ]; then return; fi
    if [ "$__idoit_in_command" -eq 1 ]; then return; fi
    __idoit_in_command=1
    mkdir -p "$__idoit_data_dir" 2>/dev/null || true
    : >|"$__idoit_stderr_file"
    exec 3>&2
    exec 2> >(tee "$__idoit_stderr_file" >&3)
}

__idoit_prompt_command() {
    local __idoit_exit=$?
    if [ "$__idoit_in_command" -eq 1 ]; then
        exec 2>&3
        exec 3>&-
        __idoit_in_command=0
    fi
    export __IDOIT_LAST_EXIT=$__idoit_exit
    export __IDOIT_LAST_STDERR="$__idoit_stderr_file"
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
#
# stderr is tee'd to $__idoit_stderr_file while the command runs; __IDOIT_LAST_STDERR is set in precmd together
# with __IDOIT_COMPLETED_CMD. Skip: idoit*, ifix, etc. (so `idoit fix` does not truncate the file).
# Limits: does not capture stderr from other terminal panes; complex multiline / nested constructs may behave
# like the parent shell only (not deeper subshells you cannot hook).

__idoit_data_dir="${XDG_DATA_HOME:-$HOME/.local/share}/idoit"
__idoit_term_log="$__idoit_data_dir/terminal_context.jsonl"
__idoit_stderr_file="$__idoit_data_dir/last-stderr-$$.txt"

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
    local cmd="$1"
    export __IDOIT_LAST_CMD="$cmd"
    case "$cmd" in
        idoit*|ido|ifix|ilearn|iexplain) return ;;
    esac
    mkdir -p "$__idoit_data_dir" 2>/dev/null || true
    : >|"$__idoit_stderr_file"
    exec {__idoit_stderr_fd}>&2
    exec 2> >(tee "$__idoit_stderr_file" >&$__idoit_stderr_fd)
}

__idoit_precmd() {
    if (( ${+__idoit_stderr_fd} )); then
        exec 2>&$__idoit_stderr_fd
        exec {__idoit_stderr_fd}>&-
        unset __idoit_stderr_fd
    fi
    export __IDOIT_LAST_EXIT=$?
    export __IDOIT_LAST_STDERR="$__idoit_stderr_file"
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
#
# Fish does not allow hooking interactive commands to tee stderr the way zsh/bash do here, so
# __IDOIT_LAST_STDERR points at a per-shell path that is usually empty — use zsh or bash for full stderr capture.
# __IDOIT_COMPLETED_CMD / __IDOIT_LAST_EXIT still come from fish_postexec.

if set -q XDG_DATA_HOME
    set -gx __idoit_data_dir "$XDG_DATA_HOME/idoit"
else
    set -gx __idoit_data_dir "$HOME/.local/share/idoit"
end
set -gx __idoit_term_log "$__idoit_data_dir/terminal_context.jsonl"
set -gx __idoit_stderr_file "$__idoit_data_dir/last-stderr-$fish_pid.txt"

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
    set -gx __IDOIT_LAST_STDERR "$__idoit_stderr_file"
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
        assert!(s.contains("__IDOIT_LAST_STDERR"));
        assert!(s.contains("tee"));
    }

    #[test]
    fn generate_zsh_uses_zsh_hooks() {
        let s = generate("zsh");
        assert!(s.contains("add-zsh-hook"));
        assert!(s.contains("__idoit_precmd"));
        assert!(s.contains("__IDOIT_LAST_STDERR"));
        assert!(s.contains("tee"));
    }

    #[test]
    fn generate_fish_uses_fish_postexec() {
        let s = generate("fish");
        assert!(s.contains("fish_postexec"));
        assert!(s.contains("function __idoit_append_terminal_session"));
        assert!(s.contains("__IDOIT_LAST_STDERR"));
    }

    #[test]
    fn generate_unknown_falls_back_to_bash_script() {
        let bash = generate("bash");
        let other = generate("some-unknown-shell");
        assert_eq!(bash, other);
    }
}
