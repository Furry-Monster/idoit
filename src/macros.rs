use std::collections::BTreeMap;
use std::path::PathBuf;

use anyhow::{Context, Result};

fn macros_path() -> PathBuf {
    crate::config::config_dir().join("macros.toml")
}

/// Previous alias storage; merged into [`load_merged`] so existing installs keep working.
fn legacy_aliases_path() -> PathBuf {
    crate::config::config_dir().join("aliases.toml")
}

fn load_file(path: &PathBuf) -> BTreeMap<String, String> {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return BTreeMap::new(),
    };
    toml::from_str(&content).unwrap_or_default()
}

/// Definitions from `macros.toml`, then any keys only in legacy `aliases.toml`.
pub fn load_merged() -> BTreeMap<String, String> {
    let mut m = load_file(&macros_path());
    for (k, v) in load_file(&legacy_aliases_path()) {
        m.entry(k).or_insert(v);
    }
    m
}

pub fn save(name: &str, body: &str) -> Result<()> {
    let mut map = load_file(&macros_path());
    map.insert(name.to_string(), body.to_string());

    let dir = crate::config::config_dir();
    std::fs::create_dir_all(&dir).with_context(|| format!("failed to create {}", dir.display()))?;

    let content = toml::to_string_pretty(&map).with_context(|| "failed to serialize macros")?;
    std::fs::write(macros_path(), content).with_context(|| "failed to write macros.toml")?;

    Ok(())
}

pub struct ExpandResult {
    pub text: String,
    /// Macro names that were expanded at least once (order of first use).
    pub used: Vec<String>,
}

fn ident_start(c: char) -> bool {
    c == '_' || c.is_ascii_alphabetic()
}

fn ident_cont(c: char) -> bool {
    c == '_' || c.is_ascii_alphanumeric()
}

fn prev_is_ident_byte(s: &str, byte_idx: usize) -> bool {
    if byte_idx == 0 {
        return false;
    }
    s[..byte_idx]
        .chars()
        .next_back()
        .is_some_and(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// Replace `@name` with the macro body. Unknown names stay as `@name`.
/// `@@` becomes a literal `@` (so you can write `@` in text).
/// `@name` is only recognized when `@` is not preceded by an identifier character (avoids `user@host`).
fn expand_one_round(s: &str, map: &BTreeMap<String, String>) -> (String, bool, Vec<String>) {
    let mut out = String::with_capacity(s.len() + 16);
    let mut changed = false;
    let mut used = Vec::new();
    let mut i = 0;

    while i < s.len() {
        let tail = &s[i..];
        let c = match tail.chars().next() {
            Some(ch) => ch,
            None => break,
        };

        if c == '@' {
            let after_at = i + c.len_utf8();
            // `@@` → literal `@` (even after an identifier, so `path@@name` works)
            if s.get(after_at..).is_some_and(|r| r.starts_with('@')) {
                out.push('@');
                i = after_at + '@'.len_utf8();
                continue;
            }
            if prev_is_ident_byte(s, i) {
                out.push('@');
                i = after_at;
                continue;
            }

            let id_start = after_at;
            let id_tail = &s[id_start..];
            let mut it = id_tail.chars();
            let Some(first) = it.next() else {
                out.push('@');
                i = after_at;
                continue;
            };
            if !ident_start(first) {
                out.push('@');
                i = after_at;
                continue;
            }

            let mut end = id_start + first.len_utf8();
            while let Some(nc) = s[end..].chars().next() {
                if !ident_cont(nc) {
                    break;
                }
                end += nc.len_utf8();
            }

            let name = &s[id_start..end];
            if let Some(replacement) = map.get(name) {
                out.push_str(replacement);
                changed = true;
                used.push(name.to_string());
                i = end;
            } else {
                out.push('@');
                out.push_str(name);
                i = end;
            }
            continue;
        }

        out.push(c);
        i += c.len_utf8();
    }

    (out, changed, used)
}

const MAX_EXPAND_ROUNDS: u32 = 32;

pub fn expand(input: &str) -> ExpandResult {
    let map = load_merged();
    let mut text = input.to_string();
    let mut used: Vec<String> = Vec::new();

    for _ in 0..MAX_EXPAND_ROUNDS {
        let (next, changed, round_used) = expand_one_round(&text, &map);
        text = next;
        for n in round_used {
            if !used.contains(&n) {
                used.push(n);
            }
        }
        if !changed {
            break;
        }
    }

    ExpandResult { text, used }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expand_literal_at() {
        let mut m = BTreeMap::new();
        m.insert("x".into(), "ok".into());
        let (s, ch, _) = expand_one_round("a@@ @x", &m);
        assert!(ch);
        assert_eq!(s, "a@ ok");
    }

    #[test]
    fn skip_email_like() {
        let mut m = BTreeMap::new();
        m.insert("g".into(), "BAD".into());
        let (s, _, _) = expand_one_round("user@gmail.com", &m);
        assert_eq!(s, "user@gmail.com");
    }

    #[test]
    fn unknown_macro_preserved() {
        let m = BTreeMap::new();
        let (s, ch, _) = expand_one_round("hi @there", &m);
        assert!(!ch);
        assert_eq!(s, "hi @there");
    }
}
