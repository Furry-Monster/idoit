//! Ordered command list from AI primary + alternates (deduped) for fix/translate selection.

/// Build display/execution order: primary first, then alternates, skipping empties and duplicates.
pub fn ordered_command_options(primary: &str, alternates: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    for c in std::iter::once(primary).chain(alternates.iter().map(String::as_str)) {
        let t = c.trim();
        if t.is_empty() {
            continue;
        }
        if !out.iter().any(|x| x == t) {
            out.push(t.to_string());
        }
    }
    if out.is_empty() {
        out.push(primary.trim().to_string());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::ordered_command_options;

    #[test]
    fn primary_only_non_empty() {
        let v = ordered_command_options("ls -la", &[]);
        assert_eq!(v, vec!["ls -la"]);
    }

    #[test]
    fn trims_and_dedupes_primary_with_alternates() {
        let v = ordered_command_options(
            "  rg foo  ",
            &["grep -r foo .".into(), "  rg foo  ".into(), "".into()],
        );
        assert_eq!(v, vec!["rg foo", "grep -r foo ."]);
    }

    #[test]
    fn empty_primary_falls_back_to_alternates_only() {
        let v = ordered_command_options("", &["a".into(), "b".into()]);
        assert_eq!(v, vec!["a", "b"]);
    }

    #[test]
    fn all_empty_yields_single_empty_string_slot() {
        let v = ordered_command_options("", &[]);
        assert_eq!(v, vec![""]);
    }
}
