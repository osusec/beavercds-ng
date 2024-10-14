// the thing that builds the stuff
// what more is there to say

use anyhow::{anyhow, Error, Result};
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
use docker::build_image;

/// Build all enabled challenges for the given profile
pub fn build_challenges(profile_name: &str) -> Result<()> {
    for chal in enabled_challenges(profile_name)? {
        build_challenge_images(profile_name, &chal);
    }

    Ok(())
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
fn build_challenge_images(profile_name: &str, chal: &ChallengeConfig) -> String {
    debug!("building images for chal {:?}", chal.directory);
    let build_infos: Vec<_> = chal
        .pods
        .iter()
        .filter_map(|c| match &c.image_source {
            Image(_) => None,
            Build(b) => Some(b),
        })
        .collect();

    for (opts, tag) in zip(build_infos, challenge_image_tags(profile_name, chal)) {
        docker::build_image(&chal.directory, opts, &tag);
    }

    "".to_string()
}

fn challenge_image_tags(profile_name: &str, chal: &ChallengeConfig) -> Vec<String> {
    let config = get_config().unwrap();

    chal.pods
        .iter()
        .map(|image| {
            format!(
                "{registry}/{challenge}-{container}:{profile}",
                registry = config.registry.domain,
                challenge = chal.name,
                container = image.name,
                profile = profile_name
            )
        })
        .collect()
}
