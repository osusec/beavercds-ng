use serde::{Deserialize, Serialize};
use serde_yaml;
use std::collections::BTreeMap;
use std::fs;

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
struct Resource {
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
	frontend_token: String,
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

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct RCDSConfig {
    flag_regex: String,
    registry: Registry,
    defaults: Defaults,
    profiles: BTreeMap<String, ProfileConfig>,
    points: Vec<ChallengePoints>,
}


fn parse_rcds_config() {
    let my_stuff = fs::read_to_string("rcds.yaml")
        .expect("Should have been able to read the file");

    let application_data: RCDSConfig = serde_yaml::from_str(&my_stuff).unwrap();
    println!("{:?}", application_data);
}
