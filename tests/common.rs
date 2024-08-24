// Shared common helper functions, mock structs, etc

use beavercds_ng::configparser::config::*;
use std::collections::BTreeMap;

/// Build empty rcds.yaml config
pub fn empty_config() -> RcdsConfig {
    RcdsConfig {
        flag_regex: "".to_string(),
        registry: Registry {
            domain: "".to_string(),
            build: UserPass {
                user: "".to_string(),
                pass: "".to_string(),
            },
            cluster: UserPass {
                user: "".to_string(),
                pass: "".to_string(),
            },
        },
        points: vec![],
        defaults: Defaults {
            difficulty: 0,
            resources: Resource {
                cpu: 0,
                memory: "0".to_string(),
            },
        },
        profiles: BTreeMap::from([(
            "default".to_string(),
            ProfileConfig {
                frontend_url: "".to_string(),
                frontend_token: None,
                challenges_domain: "".to_string(),
                kubeconfig: None,
                kubecontext: "".to_string(),
            },
        )]),
    }
}
