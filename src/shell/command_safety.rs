//! Split compound shell strings and flag high-risk patterns before execution.
//! Heuristic parser (quotes + top-level `&&`, `||`, `;`, `|`, newlines), inspired by
//! the control-operator split in claude-code-rev's Bash tooling — not a full shell grammar.

/// Join `\<newline>` continuations when the run of backslashes before `\n` is **odd**
/// (last backslash escapes the newline). Even count = each `\` pairs, newline stays a separator.
fn join_line_continuations(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut it = input.chars().peekable();

    while let Some(ch) = it.next() {
        if ch != '\\' {
            out.push(ch);
            continue;
        }
        let mut n_backslash = 1usize;
        while it.peek() == Some(&'\\') {
            it.next();
            n_backslash += 1;
        }
        if it.peek() == Some(&'\n') {
            it.next();
            if n_backslash % 2 == 1 {
                out.extend(std::iter::repeat_n('\\', n_backslash - 1));
            } else {
                out.extend(std::iter::repeat_n('\\', n_backslash));
                out.push('\n');
            }
        } else {
            out.extend(std::iter::repeat_n('\\', n_backslash));
        }
    }
    out
}

fn flush_segment(buf: &mut String, segments: &mut Vec<String>) {
    let t = buf.trim();
    if !t.is_empty() {
        segments.push(t.to_string());
    }
    buf.clear();
}

/// Splits on top-level `&&`, `||`, `;`, `|`, and newlines (outside quotes).
/// Empty input yields an empty vector.
pub fn split_compound_commands(input: &str) -> Vec<String> {
    let s = join_line_continuations(input.trim());
    if s.is_empty() {
        return vec![];
    }

    let mut segments: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut it = s.chars().peekable();
    let mut in_single = false;
    let mut in_double = false;

    while let Some(ch) = it.next() {
        if in_single {
            current.push(ch);
            if ch == '\'' {
                in_single = false;
            }
            continue;
        }
        if in_double {
            if ch == '"' {
                in_double = false;
                current.push(ch);
                continue;
            }
            if ch == '\\' {
                match it.peek().copied() {
                    Some('"') | Some('\\') | Some('$') | Some('`') => {
                        current.push(it.next().unwrap());
                    }
                    Some('\n') => {
                        it.next();
                    }
                    Some(c) => {
                        current.push('\\');
                        current.push(c);
                        it.next();
                    }
                    None => current.push('\\'),
                }
                continue;
            }
            current.push(ch);
            continue;
        }

        match ch {
            '\'' => {
                in_single = true;
                current.push(ch);
            }
            '"' => {
                in_double = true;
                current.push(ch);
            }
            '\\' => {
                if it.peek() == Some(&'\n') {
                    it.next();
                } else if let Some(c) = it.next() {
                    current.push('\\');
                    current.push(c);
                } else {
                    current.push('\\');
                }
            }
            ';' => flush_segment(&mut current, &mut segments),
            '\n' => flush_segment(&mut current, &mut segments),
            '|' => {
                if it.peek() == Some(&'|') {
                    it.next();
                }
                flush_segment(&mut current, &mut segments);
            }
            '&' => {
                if it.peek() == Some(&'&') {
                    it.next();
                    flush_segment(&mut current, &mut segments);
                } else {
                    current.push(ch);
                }
            }
            _ => current.push(ch),
        }
    }
    flush_segment(&mut current, &mut segments);
    segments
}

/// `<<` or `<<-` outside quotes (heuristic scan).
pub fn has_shell_heredoc(cmd: &str) -> bool {
    let s = join_line_continuations(cmd);
    let mut it = s.chars().peekable();
    let mut in_single = false;
    let mut in_double = false;

    while let Some(ch) = it.next() {
        if in_single {
            if ch == '\'' {
                in_single = false;
            }
            continue;
        }
        if in_double {
            if ch == '"' {
                in_double = false;
                continue;
            }
            if ch == '\\' {
                let _ = it.next();
                continue;
            }
            continue;
        }
        match ch {
            '\'' => in_single = true,
            '"' => in_double = true,
            '<' => {
                if it.peek() == Some(&'<') {
                    it.next();
                    let dash_ok = it.peek() == Some(&'-');
                    if dash_ok {
                        it.next();
                    }
                    return true;
                }
            }
            _ => {}
        }
    }
    false
}

fn collect_risk_reasons(seg: &str) -> Vec<String> {
    let mut reasons = Vec::new();
    let s = seg.trim().to_lowercase();

    let looks_like_rm = s.starts_with("rm ") || s.contains(" rm ");
    let recursive_force = s.contains("-rf")
        || s.contains("-fr")
        || s.contains("-r -f")
        || s.contains("-f -r")
        || s.contains("--recursive");
    if looks_like_rm && recursive_force {
        let dangerous_target = s.contains(" /")
            || s.contains(" /*")
            || s.contains(" /.")
            || s.ends_with(" /")
            || s.contains("--no-preserve-root")
            || s.contains('~')
            || s.contains("$home")
            || s.contains(" $HOME")
            || s.contains(" -rf *")
            || s.contains(" -fr *");
        if dangerous_target {
            reasons.push(
                "rm -rf may hit home, root, or broad globs — verify targets".to_string(),
            );
        }
    }

    if s.contains("mkfs.") || s.starts_with("mkfs ") || s.contains(" mkfs ") {
        reasons.push("mkfs can destroy filesystems on block devices".to_string());
    }

    if s.contains("dd ") && s.contains("of=/dev/") {
        reasons.push("dd with of=/dev/ can overwrite devices".to_string());
    }

    if s.contains(":(){") || s.contains(": (){") || s.contains(":() {") {
        reasons.push("possible fork-bomb style shell function — do not run blindly".to_string());
    }

    reasons
}

#[derive(Debug, Clone, Default)]
pub struct ExecSafetyReport {
    pub segments: Vec<String>,
    pub has_heredoc: bool,
    pub high_risk_reasons: Vec<String>,
}

impl ExecSafetyReport {
    pub fn analyze(cmd: &str) -> Self {
        let segments = split_compound_commands(cmd);
        let segments = if segments.is_empty() {
            vec![cmd.trim().to_string()]
        } else {
            segments
        };
        let has_heredoc = has_shell_heredoc(cmd);
        let mut high_risk_reasons: Vec<String> = segments
            .iter()
            .flat_map(|s| collect_risk_reasons(s))
            .collect();
        high_risk_reasons.sort();
        high_risk_reasons.dedup();
        Self {
            segments,
            has_heredoc,
            high_risk_reasons,
        }
    }

    pub fn needs_strict_default(&self) -> bool {
        self.segments.len() > 1
            || self.has_heredoc
            || !self.high_risk_reasons.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_simple_no_ops() {
        assert_eq!(
            split_compound_commands("ls -la"),
            vec!["ls -la".to_string()]
        );
    }

    #[test]
    fn split_double_ampersand() {
        assert_eq!(
            split_compound_commands("echo a && echo b"),
            vec!["echo a".to_string(), "echo b".to_string()]
        );
    }

    #[test]
    fn split_pipe_or_semicolon() {
        assert_eq!(
            split_compound_commands("a | b; c"),
            vec!["a".to_string(), "b".to_string(), "c".to_string()]
        );
    }

    #[test]
    fn split_does_not_break_inside_quotes() {
        assert_eq!(
            split_compound_commands("echo 'a && b' && true"),
            vec!["echo 'a && b'".to_string(), "true".to_string()]
        );
    }

    #[test]
    fn line_continuation_odd_backslashes() {
        let s = "echo hel\\\nlo";
        let j = join_line_continuations(s);
        assert!(!j.contains('\n'));
        assert!(j.contains("hello"));
    }

    #[test]
    fn heredoc_detected_outside_quotes() {
        assert!(has_shell_heredoc("cat <<EOF\nx\nEOF"));
        assert!(has_shell_heredoc("cat <<-EOF\nx\nEOF"));
    }

    #[test]
    fn heredoc_not_inside_single_quotes() {
        assert!(!has_shell_heredoc("echo '<<EOF'"));
    }

    #[test]
    fn report_multi_segment() {
        let r = ExecSafetyReport::analyze("a && b");
        assert_eq!(r.segments.len(), 2);
        assert!(r.needs_strict_default());
    }

    #[test]
    fn report_rm_rf_system_paths_flagged() {
        let r = ExecSafetyReport::analyze("rm -rf /usr/local/foo");
        assert!(!r.high_risk_reasons.is_empty());
    }

    #[test]
    fn report_rm_rf_project_dir_not_flagged() {
        let r = ExecSafetyReport::analyze("rm -rf ./target/debug");
        assert!(r.high_risk_reasons.is_empty());
    }
}
