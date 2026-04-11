//! `idoit init <shell>` — print shell integration script to stdout.

use anyhow::Result;

pub fn run(shell: &str) -> Result<()> {
    print!("{}", crate::shell::init::generate(shell));
    Ok(())
}
