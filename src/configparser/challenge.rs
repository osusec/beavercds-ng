use anyhow::{anyhow, Context, Error, Result};
use figment::providers::{Env, Format, Serialized, Yaml};
use figment::Figment;
use fully_pub::fully_pub;
use glob::glob;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use serde_nested_with::serde_nested;
use simplelog::*;
use std::collections::HashMap as Map;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use void::Void;

use crate::builder::image_tag_str;
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
    pub fn container_tag_for_pod(&self, profile_name: &str, pod_name: &str) -> Result<String> {
        let config = get_config()?;
        let pod = self
            .pods
            .iter()
            .find(|p| p.name == pod_name)
            .ok_or(anyhow!("pod {} not found in challenge", pod_name))?;

        match &pod.image_source {
            ImageSource::Image(t) => Ok(t.to_string()),
            ImageSource::Build(b) => Ok(format!(
                image_tag_str!(),
                registry = config.registry.domain,
                challenge = self.name,
                container = pod.name,
                profile = profile_name
            )),
        }
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
    File(FilePath),
    Text(FileText),
    Regex(FileRegex),
    Verifier(FileVerifier),
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[fully_pub]
struct FilePath {
    file: String,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[fully_pub]
struct FileText {
    text: String,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[fully_pub]
struct FileRegex {
    regex: String,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[fully_pub]
struct FileVerifier {
    verifier: String,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[fully_pub]
struct ProvideConfig {
    /// The challenge container image where the file should be fetched from,
    /// based on the names in `pods`. If omitted, the default value is the repo
    /// challenge directory.
    #[serde(default)]
    from: Option<String>,

    /// Rename or create zip file from included files. If only one file is
    /// included, it is renamed to this value. If multiple files are included,
    /// they are zipped into an archive with this filename. If this is omitted,
    /// each file(s) are listed individually with no renaming.
    #[serde(default, rename = "as")]
    as_file: Option<String>,

    /// List of files to read from the repo or container. If reading from the
    /// repo source files, only relative paths are supported.
    include: Vec<String>,
}
impl FromStr for ProvideConfig {
    type Err = Void;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        Ok(ProvideConfig {
            from: None,
            as_file: None,
            include: vec![s.to_string()],
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
