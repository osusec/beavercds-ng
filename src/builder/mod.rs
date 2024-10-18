// the thing that builds the stuff
// what more is there to say

use anyhow::{anyhow, Context, Error, Result};
use bollard::image::BuildImageOptions;
use futures_util::stream::Iter;
use itertools::Itertools;
use simplelog::*;
use std::default;
use std::fmt::Pointer;
use std::iter::zip;
use std::path::Path;

use crate::configparser::challenge::{BuildObject, ChallengeConfig, ImageSource::*};
use crate::configparser::{get_challenges, get_config, get_profile_config, get_profile_deploy};

pub mod docker;
use docker::{build_image, push_image};

/// Build all enabled challenges for the given profile. Returns tags built
pub fn build_challenges(profile_name: &str) -> Result<Vec<String>> {
    enabled_challenges(profile_name)?
        .iter()
        .map(|chal| build_challenge_images(profile_name, chal))
        .flatten_ok()
        .collect::<Result<_>>()
}

/// Get all enabled challenges for profile
pub fn enabled_challenges(profile_name: &str) -> Result<Vec<&ChallengeConfig>> {
    let config = get_config()?;
    let challenges = get_challenges().unwrap();
    let deploy = &get_profile_deploy(profile_name)?.challenges;

    let enabled = deploy
        .iter()
        .filter_map(|(chal, enabled)| match enabled {
            true => challenges.iter().find(|c| c.directory == Path::new(chal)),
            false => None,
        })
        .collect();

    Ok(enabled)
}

/// Build all images for challenge under given path, return image tag
fn build_challenge_images(profile_name: &str, chal: &ChallengeConfig) -> Result<Vec<String>> {
    debug!("building images for chal {:?}", chal.directory);
    let config = get_config()?;

    chal.pods
        .iter()
        .filter_map(|p| match &p.image_source {
            Image(_) => None,
            Build(b) => {
                let tag = format!(
                    "{registry}/{challenge}-{container}:{profile}",
                    registry = config.registry.domain,
                    challenge = chal.name,
                    container = p.name,
                    profile = profile_name
                );
                Some(
                    docker::build_image(&chal.directory, b, &tag).with_context(|| {
                        format!(
                            "error building image {} for chal {}",
                            p.name,
                            chal.directory.to_string_lossy()
                        )
                    }),
                )
            }
        })
        .collect::<Result<_>>()
}

/// Push passed tags to registry
pub fn push_tags(tags: Vec<String>) -> Result<Vec<String>> {
    let config = get_config()?;

    let built_tags = tags
        .iter()
        .map(|tag| {
            push_image(tag, &config.registry.build)
                .with_context(|| format!("error pushing image {tag}"))
        })
        .collect::<Result<_>>()?;

    Ok(built_tags)
}
