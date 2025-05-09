use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, bail, Context, Error, Ok, Result};
use itertools::Itertools;
use tracing::{debug, error, info, trace, warn};

use crate::builder::BuildResult;
use crate::configparser::challenge::{ExposeType, FlagType};
use crate::configparser::config::ProfileConfig;
use crate::configparser::{enabled_challenges, get_config, get_profile_config, ChallengeConfig};
use crate::utils::render_strict;

use super::kubernetes::KubeDeployResult;
use super::s3::S3DeployResult;

/// Sync deployed challenges with rCTF frontend
pub async fn update_frontend(
    profile_name: &str,
    chal: &ChallengeConfig,
    build_result: &BuildResult,
    kube_result: &KubeDeployResult,
    s3_result: &S3DeployResult,
) -> Result<String> {
    let profile = get_profile_config(profile_name)?;
    let enabled_challenges = enabled_challenges(profile_name)?;

    // TODO: hook this up to real frontend! Waiting on rCTF frontend reimplementation

    // for now, render out all challenge information to a markdown file for
    // admins to enter manually

    let hostname = chal_domain(chal, &profile.challenges_domain);
    let rendered_desc = render_strict(
        &chal.description,
        minijinja::context! {
            challenge => chal,
            host => hostname,
            hostname => hostname,
            port => chal_port(chal),
            nc => format!("`nc {} {}`", hostname, chal_port(chal)),
            url => format!("[https://{hostname}](https://{hostname})", ),
            link => format!("https://{hostname}"),
        },
    )?;

    // urls to markdown links
    let asset_urls = s3_result
        .uploaded_asset_urls
        .iter()
        .map(|url| {
            format!(
                "[{}]({})",
                Path::new(url)
                    .file_name()
                    .expect("asset URL has no path!")
                    .to_string_lossy(),
                url
            )
        })
        .join("\n\n");
    let flag = match &chal.flag {
        FlagType::RawString(f) => f.clone(),
        FlagType::File { file } => {
            let full_path = chal.directory.join(file);
            let mut flag = String::new();
            let f = File::open(&full_path)
                .with_context(|| {
                    format!(
                        "could not open flag file {:?} for challenge {:?}",
                        &full_path, chal.directory
                    )
                })?
                .read_to_string(&mut flag);
            flag
        }
        FlagType::Text { text } => text.clone(),
        FlagType::Regex { regex } => unimplemented!(),
        FlagType::Verifier { verifier } => unimplemented!(),
    };

    let info_md = format!(
        r"
##  `{slug}`

|        |   |
--------:|---|
name     | `{name}`
category | `{cat}`
author   | `{author}`

### description

```
{desc}

{asset_urls}
```

### flag

`{flag}`

---
",
        slug = chal.slugify_slash(),
        name = chal.name,
        cat = chal.category,
        author = chal.author,
        desc = rendered_desc,
        asset_urls = asset_urls,
        flag = flag.trim(),
    );

    // TODO: proper frontend updates

    Ok(info_md)
}

// TODO: move to impl ChallengeConfig?
// TODO: return Option and report errors when missing
fn chal_domain(chal: &ChallengeConfig, chal_domain: &str) -> String {
    // find first container with expose
    match chal.pods.iter().find(|p| !p.ports.is_empty()) {
        Some(p) => {
            let subdomain = match &p.ports[0].expose {
                ExposeType::Tcp(_port) => &chal.slugify_name(),
                ExposeType::Http(hostname) => hostname,
            };
            format!("{subdomain}.{chal_domain}")
        }
        // no pods have expose, no hostname for challenge
        None => "".to_string(),
    }
}

fn chal_port(chal: &ChallengeConfig) -> &i64 {
    // find first container with expose
    match chal.pods.iter().find(|p| !p.ports.is_empty()) {
        Some(p) => match &p.ports[0].expose {
            ExposeType::Tcp(port) => port,
            ExposeType::Http(_hostname) => &443,
        },
        // no pods have expose, no hostname for challenge
        None => &0,
    }
}
