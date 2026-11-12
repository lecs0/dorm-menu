pub mod auth;
pub mod http;
pub mod parser;
pub mod scraper;
pub mod storage;
pub mod update;

#[cfg(all(
    feature = "nas-legacy-clock",
    target_arch = "arm",
    target_env = "musl",
    target_endian = "little"
))]
mod legacy_clock;
