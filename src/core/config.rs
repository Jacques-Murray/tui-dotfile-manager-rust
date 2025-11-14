// Author: Jacques Murray
//! Defines the strongly-typed configuration structs using Serde.

use serde::Deserialize;
use std::collections::HashMap;
use std::path::PathBuf;

/// The root configuration struct, mapping to the TOML file.
#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub settings: Settings,
    pub profiles: HashMap<String, Profile>,
}

/// The [settings] section of the config.
#[derive(Debug, Deserialize, Clone)]
pub struct Settings {
    pub repo_dir: PathBuf,
    pub backup_dir: PathBuf,
}

/// A single profile, e.g., [profiles.work]
#[derive(Debug, Deserialize, Clone)]
pub struct Profile {
    pub links: Vec<Link>,
}

/// A single link task within a profile.
#[derive(Debug, Deserialize, Clone)]
pub struct Link {
    pub source: PathBuf,
    pub target: PathBuf,
}
