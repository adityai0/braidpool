use serde::{
    de::{self, Deserializer},
    Deserialize,
};
use std::path::PathBuf;
use std::time::Duration;

/// Deserialize a duration from a TOML string.
pub fn duration_from_toml<'de, D>(deserializer: D) -> Result<Duration, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::Deserialize;

    #[derive(serde::Deserialize)]
    struct Helper {
        unit: String,
        value: u64,
    }

    let helper = Helper::deserialize(deserializer)?;
    match helper.unit.as_str() {
        "seconds" => Ok(Duration::from_secs(helper.value)),
        "secs" => Ok(Duration::from_secs(helper.value)),
        "s" => Ok(Duration::from_secs(helper.value)),
        "milliseconds" => Ok(Duration::from_millis(helper.value)),
        "millis" => Ok(Duration::from_millis(helper.value)),
        "ms" => Ok(Duration::from_millis(helper.value)),
        "microseconds" => Ok(Duration::from_micros(helper.value)),
        "micros" => Ok(Duration::from_micros(helper.value)),
        "us" => Ok(Duration::from_micros(helper.value)),
        "nanoseconds" => Ok(Duration::from_nanos(helper.value)),
        "nanos" => Ok(Duration::from_nanos(helper.value)),
        "ns" => Ok(Duration::from_nanos(helper.value)),
        // ... add other units as needed
        _ => Err(serde::de::Error::custom("Unsupported duration unit")),
    }
}

/// Deserialize an optional TOML string into `Option<PathBuf>`, expanding:
/// - `~` (home directory)
/// - environment variables like `$HOME` or `${VAR}`
///
/// Use this for **optional** path fields with:
/// `#[serde(default, deserialize_with = "opt_path_from_toml")]`.
///
/// - Missing field → `None`
/// - Present field → `Some(PathBuf)`
///
/// Fails if the field is present but not a string, or if expansion fails.
pub fn opt_path_from_toml<'de, D>(deserializer: D) -> Result<Option<PathBuf>, D::Error>
where
    D: Deserializer<'de>,
{
    let opt = Option::<String>::deserialize(deserializer)?;

    opt.map(|raw| {
        let expanded = shellexpand::full(&raw).map_err(|e| de::Error::custom(e.to_string()))?;
        Ok(PathBuf::from(expanded.to_string()))
    })
    .transpose()
}
