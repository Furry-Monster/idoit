use std::path::PathBuf;
use std::time::{Duration, SystemTime};

use sha2::{Digest, Sha256};

use crate::config::settings::CacheSettings;

#[allow(dead_code)]
fn cache_dir() -> PathBuf {
    dirs::cache_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("idoit")
        .join("responses")
}

#[allow(dead_code)]
fn cache_key(system: &str, user_message: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(system.as_bytes());
    hasher.update(b"|");
    hasher.update(user_message.as_bytes());
    hex::encode(hasher.finalize())
}

#[allow(dead_code)]
pub fn get(settings: &CacheSettings, system: &str, user_message: &str) -> Option<String> {
    if !settings.enabled {
        return None;
    }

    let key = cache_key(system, user_message);
    let path = cache_dir().join(&key);

    let metadata = std::fs::metadata(&path).ok()?;
    let modified = metadata.modified().ok()?;
    let age = SystemTime::now().duration_since(modified).ok()?;

    if age > Duration::from_secs(settings.ttl_secs) {
        let _ = std::fs::remove_file(&path);
        return None;
    }

    std::fs::read_to_string(&path).ok()
}

#[allow(dead_code)]
pub fn put(settings: &CacheSettings, system: &str, user_message: &str, response: &str) {
    if !settings.enabled {
        return;
    }

    let dir = cache_dir();
    let _ = std::fs::create_dir_all(&dir);

    let key = cache_key(system, user_message);
    let _ = std::fs::write(dir.join(&key), response);
}
