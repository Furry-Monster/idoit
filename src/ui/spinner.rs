use indicatif::{ProgressBar, ProgressStyle};
use std::time::Duration;

pub struct Spinner {
    bar: ProgressBar,
}

impl Spinner {
    pub fn new(message: &str) -> Self {
        let bar = ProgressBar::new_spinner();
        bar.set_style(
            ProgressStyle::default_spinner()
                .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"])
                .template("  {spinner} {msg}")
                .unwrap(),
        );
        bar.set_message(message.to_string());
        bar.enable_steady_tick(Duration::from_millis(80));
        Self { bar }
    }

    pub fn finish(self) {
        self.bar.finish_and_clear();
    }
}
