use anyhow::Result;
use dialoguer::Confirm;

pub fn confirm_execution(auto_yes: bool) -> Result<bool> {
    if auto_yes {
        return Ok(true);
    }

    let confirmed = Confirm::new()
        .with_prompt("  Execute?")
        .default(true)
        .interact()?;

    Ok(confirmed)
}

pub fn confirm_anyway() -> Result<bool> {
    let confirmed = Confirm::new()
        .with_prompt("  This may install or modify things. Continue?")
        .default(false)
        .interact()?;

    Ok(confirmed)
}
