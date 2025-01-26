// the thing that builds the stuff
// what more is there to say

use anyhow::{anyhow, Context, Error, Result};
use bollard::image::BuildImageOptions;
use futures::stream::Iter;
use itertools::Itertools;
use simplelog::*;
use std::default;
use std::fmt::Pointer;
use std::iter::zip;
use std::path::{Path, PathBuf};

use crate::configparser::challenge::{
    BuildObject, ChallengeConfig, ImageSource::*, Pod, ProvideConfig,
};
use crate::configparser::{enabled_challenges, get_config};

pub mod docker;

pub mod artifacts;
use artifacts::extract_asset;

// define tag format as reusable macro
macro_rules! image_tag_str {
    () => {
        "{registry}/{challenge}-{container}:{profile}"
    };
}

/// Information about all of a challenge's build artifacts.
#[derive(Debug)]
pub struct BuildResult {
    /// Container image tags of all containers in the challenge, if the challenge has container images.
    /// Will be empty if challenge has no images built from source.
    tags: Vec<String>,
    /// Path on disk to local assets (both built and static).
    /// Will be empty if challenge has no file assets
    assets: Vec<PathBuf>,
}

/// Build all enabled challenges for the given profile. Returns tags built
pub fn build_challenges(
    profile_name: &str,
    push: bool,
    extract_artifacts: bool,
) -> Result<Vec<BuildResult>> {
    enabled_challenges(profile_name)?
        .iter()
        .map(|chal| build_challenge(profile_name, chal, push, extract_artifacts))
        .collect::<Result<_>>()
}

/// Build all images from given challenge, optionally pushing image or extracting artifacts
fn build_challenge(
    profile_name: &str,
    chal: &ChallengeConfig,
    push: bool,
    extract_artifacts: bool,
) -> Result<BuildResult> {
    debug!("building images for chal {:?}", chal.directory);
    let config = get_config()?;

    let mut built = BuildResult {
        tags: vec![],
        assets: vec![],
    };

    built.tags = chal
        .pods
        .iter()
        .filter_map(|p| match &p.image_source {
            // ignore any pods that use existing images
            Image(_) => None,
            // build any pods that need building
            Build(b) => {
                let tag = format!(
                    image_tag_str!(),
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
        .collect::<Result<_>>()?;

    if push {
        debug!(
            "pushing {} tags for chal {:?}",
            built.tags.len(),
            chal.directory
        );

        built
            .tags
            .iter()
            .map(|tag| {
                docker::push_image(tag, &config.registry.build)
                    .with_context(|| format!("error pushing image {tag}"))
            })
            .collect::<Result<Vec<_>>>()?;
    }

    if extract_artifacts {
        info!("extracting build artifacts for chal {:?}", chal.directory);

        // find the matching tag for Provide entries that have a `from:` source
        let image_assoc = chal
            .provide
            .iter()
            .filter_map(|p| {
                p.from.as_ref().map(|f| {
                    (
                        p,
                        format!(
                            image_tag_str!(),
                            registry = config.registry.domain,
                            challenge = chal.name,
                            container = f,
                            profile = profile_name
                        ),
                    )
                })
            })
            .collect_vec();

        built.assets = image_assoc
            .into_iter()
            .map(|(p, tag)| {
                let name = format!(
                    "asset-container-{}-{}",
                    chal.directory.to_string_lossy().replace("/", "-"),
                    p.from.clone().unwrap()
                );
                let container = docker::create_container(&tag, &name)?;

                let asset_result = extract_asset(chal, p, &container).with_context(|| {
                    format!(
                        "failed to extract build artifacts for chal {:?} container {:?}",
                        chal.directory,
                        p.from.clone().unwrap()
                    )
                });

                // clean up container even if it failed
                docker::remove_container(container)?;

                asset_result
            })
            .flatten_ok()
            .collect::<Result<Vec<_>>>()?;

        info!("extracted artifacts: {:?}", built.assets);
    }

    Ok(built)
}
