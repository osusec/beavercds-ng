use anyhow::{Context, Result};
use fully_pub::fully_pub;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use simplelog::*;
use std::collections::HashMap as Map;
use std::fs;

use figment::providers::{Env, Format, Yaml};
use figment::Figment;

pub fn parse() -> Result<RcdsConfig> {
    debug!("trying to parse rcds.yaml");

    let env_overrides = Env::prefixed("BEAVERCDS_").split("_").map(|var| {
        // Using "_" as the split character works for almost all of our keys.
        // but some of the profile settings keys have underscores as part of the
        // key. This handles those few keys by undoing the s/_/./ that the
        // Figment split() did.
        var.to_string()
            .to_lowercase()
            .replace("frontend.", "frontend_")
            .replace("challenges.", "challenges_")
            .into()
    });
    trace!(
        "overriding config with envvar values: {}",
        env_overrides
            .iter()
            .map(|(key, val)| format!("{}='{}'", key.string, val))
            .join(", ")
    );

    let config = Figment::from(Yaml::file("rcds.yaml"))
        .merge(env_overrides)
        .extract()
        .with_context(|| "failed to parse rcds.yaml")?;

    trace!("got config: {config:#?}");

    Ok(config)
}

//
// ==== Structs for rcds.yaml parsing ====
//

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[fully_pub]
struct RcdsConfig {
    flag_regex: String,
    registry: Registry,
    defaults: Defaults,
    deploy: Map<String, ProfileDeploy>,
    profiles: Map<String, ProfileConfig>,
    points: Vec<ChallengePoints>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[fully_pub]
struct Registry {
    domain: String,
    build: UserPass,
    cluster: UserPass,
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
#[fully_pub]
struct UserPass {
    user: String,
    pass: String,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[fully_pub]
struct Resource {
    cpu: i64,
    memory: String,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[fully_pub]
struct Defaults {
    difficulty: i64,
    resources: Resource,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[fully_pub]
struct ProfileDeploy {
    #[serde(flatten)]
    challenges: Map<String, bool>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[fully_pub]
struct ProfileConfig {
    // deployed_challenges: HashMap<String, bool>,
    frontend_url: String,
    frontend_token: String,
    challenges_domain: String,
    kubeconfig: Option<String>,
    kubecontext: String,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[fully_pub]
struct ChallengePoints {
    difficulty: i64,
    min: i64,
    max: i64,
}
