use anyhow::{anyhow, Context, Error, Result};
use figment::providers::{Env, Format, Serialized, Yaml};
use figment::Figment;
use fully_pub::fully_pub;
use glob::glob;
use itertools::Itertools;
use minijinja::render;
use serde::{Deserialize, Serialize};
use serde_nested_with::serde_nested;
use std::collections::HashMap as Map;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use tracing::{debug, error, info, trace, warn};
use void::Void;

use crate::configparser::config::Resource;
use crate::configparser::field_coersion::string_or_struct;
use crate::configparser::get_config;

pub fn parse_all() -> Result<Vec<ChallengeConfig>, Vec<Error>> {
    // find all challenge.yaml files
    // only look for paths two entries deep (i.e. always at `<category>/<name>/challenge.yaml`)
    let (challenges, parse_errors): (Vec<_>, Vec<_>) = glob("*/*/challenge.yaml")
        .unwrap() // static pattern so will never error
        // try to parse each one
        .map(|glob_result| match glob_result {
            Ok(path) => parse_one(&path)
                .with_context(|| format!("failed to parse challenge config {:?}", path)),
            Err(e) => Err(e.into()),
        })
        .partition_result();

    trace!(
        "parsed chals: {:?}",
        challenges
            .iter()
            .map(|c| format!("{}/{}", c.category, c.name))
            .collect::<Vec<_>>()
    );
    debug!(
        "parsed {} chals, {} others failed parsing",
        challenges.len(),
        parse_errors.len()
    );

    if parse_errors.is_empty() {
        Ok(challenges)
    } else {
        Err(parse_errors)
    }
}

pub fn parse_one(path: &PathBuf) -> Result<ChallengeConfig> {
    trace!("trying to parse {path:?}");

    // remove 'challenge.yaml' from path
    let chal_dir = path
        .parent()
        .expect("could not extract path from search path");

    // extract category from challenge path
    let category = chal_dir
        .components()
        .nth_back(1)
        .expect("could not find category from path")
        .as_os_str()
        .to_str()
        .unwrap();

    let mut parsed: ChallengeConfig = Figment::new()
        .merge(Yaml::file(path.clone()))
        // merge in generated data from file path
        .merge(Serialized::default("directory", chal_dir))
        .merge(Serialized::default("category", category))
        .extract()?;

    // coerce pod env lists to maps
    // TODO: do this in serde deserialize?
    for pod in parsed.pods.iter_mut() {
        pod.env = match pod.env.clone() {
            ListOrMap::Map(m) => ListOrMap::Map(m),
            ListOrMap::List(l) => {
                // split NAME=VALUE list into separate name and value
                let split: Vec<(String, String)> = l
                    .into_iter()
                    .map(|var| {
                        // error if envvar is malformed
                        let split = var.splitn(2, '=').collect_vec();
                        if split.len() == 2 {
                            Ok((split[0].to_string(), split[1].to_string()))
                        } else {
                            Err(anyhow!("Cannot split envvar {var:?}"))
                        }
                    })
                    .collect::<Result<_>>()?;
                // build hashmap from split name and value iteratively. this
                // can't use HashMap::from() here since the values are dynamic
                // and from() only works for Vec constants
                let map = split
                    .into_iter()
                    .fold(Map::new(), |mut map, (name, value)| {
                        map.insert(name, value);
                        map
                    });
                ListOrMap::Map(map)
            }
        }
    }

    trace!("got challenge config: {parsed:#?}");

    Ok(parsed)
}

//
// ==== Structs for challenge.yaml parsing ====
//

#[serde_nested]
#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[fully_pub]
pub struct ChallengeConfig {
    name: String,
    author: String,
    description: String,
    category: String,

    directory: PathBuf,

    #[serde(default = "default_difficulty")]
    difficulty: i64,

    flag: FlagType,

    #[serde(default)]
    // map deserialize_with to type in vec
    #[serde_nested(sub = "ProvideConfig", serde(deserialize_with = "string_or_struct"))]
    provide: Vec<ProvideConfig>, // optional if no files provided

    #[serde(default)]
    pods: Vec<Pod>, // optional if no containers used
}
impl ChallengeConfig {
    /// Return the container image tag for the pod; either the upstream image or
    /// the tag to be built if the image is to be built from source.
    pub fn container_tag_for_pod(&self, profile_name: &str, pod_name: &str) -> Result<String> {
        let config = get_config()?;
        let pod = self
            .pods
            .iter()
            .find(|p| p.name == pod_name)
            .ok_or(anyhow!("pod {} not found in challenge", pod_name))?;

        match &pod.image_source {
            ImageSource::Image(t) => Ok(t.to_string()),
            // render image tag template from config
            ImageSource::Build(b) => Ok(render!(
                &get_config()?.registry.tag_format,
                domain => config.registry.domain,
                challenge => self.slugify(),
                container => pod.name,
                profile => profile_name
            )),
        }
    }

    /// Create challenge category-name slug from directory path
    pub fn slugify(&self) -> String {
        self.slugify_slash().replace("/", "-")
    }

    /// Create challenge category/name slug from directory path, with category slash
    pub fn slugify_slash(&self) -> String {
        self.directory
            .to_string_lossy()
            .to_lowercase()
            .split_whitespace()
            .join("-")
    }
}

fn default_difficulty() -> i64 {
    1
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
#[fully_pub]
enum FlagType {
    RawString(String),
    File { file: PathBuf },
    Text { text: String },
    Regex { regex: String },
    Verifier { verifier: String },
}

// Parse each distinct kind of Provide action as a separate enum variant
// TODO: enforce relative/absolute paths for repo/container Provide's (`validator` crate?)
#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged, deny_unknown_fields)]
#[fully_pub]
enum ProvideConfig {
    /// Upload file(s) as-is.
    /// Single or multiple files with no as: or from:
    /// Default if only a string is given.
    FromRepo {
        #[serde(rename = "include")]
        files: Vec<PathBuf>,
    },
    /// Rename single file before uploading.
    /// Single file with as: field without from:
    FromRepoRename {
        #[serde(rename = "include")]
        from: PathBuf,
        #[serde(rename = "as")]
        to: PathBuf,
    },
    /// Upload multiple files in zip archive
    /// Multiple files with as: field without from:
    FromRepoArchive {
        #[serde(rename = "include")]
        files: Vec<PathBuf>,
        #[serde(rename = "as")]
        archive_name: PathBuf,
    },

    /// Upload file(s) from container as-is.
    /// Single or multiple files with no as:
    FromContainer {
        #[serde(rename = "from")]
        container: String,
        #[serde(rename = "include")]
        files: Vec<PathBuf>,
    },
    /// Rename single file from container before uploading.
    /// Single file with as: field
    FromContainerRename {
        #[serde(rename = "from")]
        container: String,
        #[serde(rename = "include")]
        from: PathBuf,
        #[serde(rename = "as")]
        to: PathBuf,
    },
    /// Upload multiple files from container in zip archive
    /// Multiple files with as: field
    FromContainerArchive {
        #[serde(rename = "from")]
        container: String,
        #[serde(rename = "include")]
        files: Vec<PathBuf>,
        #[serde(rename = "as")]
        archive_name: PathBuf,
    },
}
impl FromStr for ProvideConfig {
    type Err = Void;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        Ok(ProvideConfig::FromRepo {
            files: vec![PathBuf::from(s)],
        })
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[fully_pub]
struct Pod {
    name: String,

    #[serde(flatten)]
    image_source: ImageSource,

    #[serde(default)]
    env: ListOrMap,

    resources: Option<Resource>,
    replicas: i64,
    ports: Vec<PortConfig>,
    volume: Option<String>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[fully_pub]
enum ImageSource {
    #[serde(deserialize_with = "string_or_struct")]
    Build(BuildObject),
    Image(String),
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[fully_pub]
struct BuildObject {
    context: String,
    #[serde(default = "default_dockerfile")]
    dockerfile: String,
    // dockerfile_inline: String,
    #[serde(default)]
    args: Map<String, String>,
}
impl FromStr for BuildObject {
    type Err = Void;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        Ok(BuildObject {
            context: s.to_string(),
            dockerfile: default_dockerfile(),
            args: Default::default(),
        })
    }
}
fn default_dockerfile() -> String {
    "Dockerfile".to_string()
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
#[serde(untagged)]
#[fully_pub]
enum ListOrMap {
    List(Vec<String>),
    Map(Map<String, String>),
}
impl Default for ListOrMap {
    fn default() -> Self {
        ListOrMap::Map(Map::new())
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[fully_pub]
struct PortConfig {
    internal: i64,
    expose: ExposeType,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[fully_pub]
enum ExposeType {
    Tcp(i64),
    Http(String),
}
