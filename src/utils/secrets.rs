use std::env;

pub fn read_env(key: &str) -> Option<String> {
    env::var(key).ok()
}
