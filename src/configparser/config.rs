use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;

use anyhow::{Context, Result};
use simplelog::*;

pub fn parse() -> Result<RcdsConfig> {
    trace!("trying to parse rcds.yaml");

    let contents = fs::read_to_string("rcds.yaml").with_context(|| "failed to read rcds.yaml")?;
    let parsed = serde_yaml::from_str(&contents).with_context(|| "failed to parse rcds.yaml")?;

    Ok(parsed)
}

//
// ==== Structs for rcds.yaml parsing ====
//

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct RcdsConfig {
    flag_regex: String,
    registry: Registry,
    defaults: Defaults,
    profiles: BTreeMap<String, ProfileConfig>,
    points: Vec<ChallengePoints>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
enum Registry {
    TopLevel(RegistryOne),
    Nested(RegistryTwo),
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct RegistryOne {
    domain: String,
    build: UserPass,
    cluster: UserPass,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct RegistryTwo {
    domain: String,
    user: String,
    pass: String,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct UserPass {
    user: String,
    pass: String,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Resource {
    cpu: i64,
    memory: String,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct Defaults {
    difficulty: i64,
    resources: Resource,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct ProfileConfig {
    // deployed_challenges: BTreeMap<String, bool>,
    frontend_url: String,
    frontend_token: Option<String>,
    challenges_domain: String,
    kubeconfig: String,
    kubecontext: String,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct ChallengePoints {
    difficulty: i64,
    min: i64,
    max: i64,
}
