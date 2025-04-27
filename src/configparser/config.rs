use anyhow::{Context, Result};
use fully_pub::fully_pub;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::collections::HashMap as Map;
use std::fs;
use tracing::{debug, error, info, trace, warn};

use figment::providers::{Env, Format, Yaml};
use figment::Figment;

pub fn parse() -> Result<RcdsConfig> {
    debug!("trying to parse rcds.yaml");

    let env_overrides = Env::prefixed("BEAVERCDS_").split("_").map(|var| {
        // Using "_" as the split character works for almost all of our keys,
        // but some profile settings have underscores. This handles those few
        // keys by undoing the s/_/./ that the figment::split() did.
        var.to_string()
            .to_lowercase()
            .replace("frontend.", "frontend_")
            .replace("challenges.", "challenges_")
            .replace("s3.access.", "s3.access_")
            .replace("s3.secret.", "s3.secret_")
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
    /// Registry base url used to build part of the full image string.
    ///
    /// Example: `domain: "registry.io/myctf"`
    domain: String,

    /// Image tag format string. Useful if the registry forces a single
    /// container repository. (AWS...)
    ///
    /// Format: Jinja-style double-braces around field name (`{{ field_name }}`)
    ///
    /// Default: `"{{domain}}/{{challenge}}-{{container}}:{{profile}}"`
    ///
    /// Available fields:
    /// - `domain`: the domain config field above; the repository base URL
    /// - `challenge`: challenge name, slugified
    /// - `container`: name of the specific pod in the challenge this image is for
    /// - `profile`: the current deployment profile, for isolating images between environments
    ///
    /// Example:
    ///
    /// For challenge `pwn/notsh`, chal pod container `main`, profile `prod`, and the example domain:
    /// ```py
    /// the default --> "registry.io/myctf/pwn-notsh-main:prod"
    ///
    /// "{{domain}}:{{challenge}}-{{container}}" --> "registry.io/myctf:pwn-notsh-main"
    /// ```
    #[serde(default = "default_tag_format")]
    tag_format: String,

    /// Container registry login for pushing images during build/deploy
    build: UserPass,
    /// Container registry login for pulling images in cluster. Can and should be read-only.
    cluster: UserPass,
}
fn default_tag_format() -> String {
    "{{domain}}/{{challenge}}-{{container}}:{{profile}}".to_string()
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
    s3: S3Config,
    dns: serde_yml::Value,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[fully_pub]
struct ChallengePoints {
    difficulty: i64,
    min: i64,
    max: i64,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[fully_pub]
struct S3Config {
    bucket_name: String,
    endpoint: String,
    region: String,
    access_key: String,
    secret_key: String,
}
