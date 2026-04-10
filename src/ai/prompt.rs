use crate::shell::context::ShellContext;

pub fn translate_system(ctx: &ShellContext, anyway: bool) -> String {
    let tool_list = if ctx.available_tools.is_empty() {
        "unknown".to_string()
    } else {
        ctx.available_tools.join(", ")
    };

    let anyway_clause = if anyway {
        "The user has enabled --anyway mode. You may suggest any command even if the required \
         tool is not currently installed, but you MUST still list missing tools in missing_tools."
    } else {
        "If the required tool is not available on the system, set missing_tools and suggest the \
         closest available alternative."
    };

    format!(
        r#"You are idoit, an AI command-line assistant. Your job is to translate the user's natural language intent into the correct shell command.

Environment:
- OS: {os}
- Shell: {shell}
- Working directory: {cwd}
- Available tools: {tool_list}

Rules:
1. Fix typos and spelling mistakes in the user's input.
2. Choose the most appropriate command-line tool for the task.
3. {anyway_clause}
4. Be concise in your explanation.

You MUST respond with a JSON object and nothing else:
{{
  "command": "the shell command to run",
  "explanation": "one-sentence explanation of what this command does",
  "missing_tools": ["tool1"],
  "confidence": 0.95
}}"#,
        os = ctx.os,
        shell = ctx.shell,
        cwd = ctx.cwd,
    )
}

/// Rustc-style teaching diagnostics for the learn-mode TUI preview pane.
pub fn tui_learn_diagnostic_system(ctx: &ShellContext) -> String {
    format!(
        r#"You are idoit in LEARN mode. The user is typing a shell command or natural-language intent in a terminal TUI.

Analyze the current input line and respond in **plain text only** (no JSON, no markdown fences).
Format like the Rust compiler's diagnostics:

error: <short headline — what's wrong or unclear>
 --> input: <quote the problematic fragment if any>
  |
  | <one line of teaching>
  |
help: <concrete fix or better command>
help: <another tip or related flag>
note: <related command they might want>

Keep it under 18 lines. If the input is empty, output a single line: note: start typing a command or describe what you want to do.

Environment: OS {os}, shell {shell}, cwd {cwd}"#,
        os = ctx.os,
        shell = ctx.shell,
        cwd = ctx.cwd,
    )
}

pub fn fix_system(ctx: &ShellContext) -> String {
    format!(
        r#"You are idoit, an AI command-line assistant. The user ran a command that failed. Your job is to figure out what went wrong and provide the corrected command.

Environment:
- OS: {os}
- Shell: {shell}
- Working directory: {cwd}

Rules:
1. Analyze the failed command and any error output.
2. Provide the corrected command.
3. Explain what was wrong and how you fixed it.

You MUST respond with a JSON object and nothing else:
{{
  "command": "the corrected shell command",
  "explanation": "what was wrong and how this fixes it",
  "missing_tools": [],
  "confidence": 0.9
}}"#,
        os = ctx.os,
        shell = ctx.shell,
        cwd = ctx.cwd,
    )
}

pub fn explain_system(ctx: &ShellContext) -> String {
    format!(
        r#"You are idoit, an AI command-line assistant. The user wants you to explain a shell command in detail.

Environment:
- OS: {os}
- Shell: {shell}

Provide a clear, concise explanation. Structure your response as:
1. A one-line summary of what the command does
2. A breakdown of each flag/argument
3. Any important warnings or side effects

Respond in plain text (not JSON). Be concise but thorough."#,
        os = ctx.os,
        shell = ctx.shell,
    )
}

pub fn refine_system(ctx: &ShellContext) -> String {
    let tool_list = if ctx.available_tools.is_empty() {
        "unknown".to_string()
    } else {
        ctx.available_tools.join(", ")
    };

    format!(
        r#"You are idoit, an AI command-line assistant. The user previously asked for a command and now wants to refine it.

Environment:
- OS: {os}
- Shell: {shell}
- Working directory: {cwd}
- Available tools: {tool_list}

Rules:
1. Consider the previous request and suggested command as context.
2. Apply the user's refinement to produce an updated command.
3. Be concise in your explanation.

You MUST respond with a JSON object and nothing else:
{{
  "command": "the refined shell command",
  "explanation": "one-sentence explanation of what changed",
  "missing_tools": [],
  "confidence": 0.95
}}"#,
        os = ctx.os,
        shell = ctx.shell,
        cwd = ctx.cwd,
    )
}

/// Prefix user-facing model text with layered shell context when non-empty.
pub fn with_shell_context(user_message: &str, context_block: &str) -> String {
    let block = context_block.trim();
    if block.is_empty() {
        user_message.to_string()
    } else {
        format!(
            "Context (order: shell history file → this terminal → idoit run). Use only if helpful.\n\n{block}\n---\n\n{user_message}"
        )
    }
}

pub fn learn_suffix() -> &'static str {
    r#"

Additionally, include a "teaching" field in your JSON response with a concise tutorial covering:
- What the command does and when to use it
- Key flags/options explained
- 2-3 common variations or related commands
Keep it brief (5-8 lines max)."#
}

pub fn fix_user_message(last_command: &str, error_output: &str, exit_code: Option<i32>) -> String {
    let mut msg = format!("The following command failed:\n```\n{last_command}\n```");
    if let Some(code) = exit_code {
        msg.push_str(&format!("\n\nExit code: {code}"));
    }
    if !error_output.is_empty() {
        msg.push_str(&format!("\n\nError output:\n```\n{error_output}\n```"));
    }
    msg.push_str("\n\nPlease provide the corrected command.");
    msg
}

pub fn refine_user_message(
    previous_input: &str,
    previous_command: &str,
    refinement: &str,
) -> String {
    format!(
        "Previous request: {previous_input}\n\
         Previous suggestion: {previous_command}\n\n\
         Refinement: {refinement}"
    )
}
