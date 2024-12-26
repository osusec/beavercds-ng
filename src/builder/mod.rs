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
use std::path::Path;

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

/// Build all enabled challenges for the given profile. Returns tags built
pub fn build_challenges(
    profile_name: &str,
    push: bool,
    extract_artifacts: bool,
) -> Result<Vec<String>> {
    enabled_challenges(profile_name)?
        .iter()
        .map(|chal| build_challenge(profile_name, chal, push, extract_artifacts))
        .flatten_ok()
        .collect::<Result<_>>()
}

/// Build all images from given challenge, optionally pushing image or extracting artifacts
fn build_challenge(
    profile_name: &str,
    chal: &ChallengeConfig,
    push: bool,
    extract_artifacts: bool,
) -> Result<Vec<String>> {
    debug!("building images for chal {:?}", chal.directory);
    let config = get_config()?;

    let built_tags: Vec<_> = chal
        .pods
        .iter()
        .filter_map(|p| match &p.image_source {
            Image(_) => None,
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
            built_tags.len(),
            chal.directory
        );

        built_tags
            .iter()
            .map(|tag| {
                docker::push_image(tag, &config.registry.build)
                    .with_context(|| format!("error pushing image {tag}"))
            })
            .collect::<Result<Vec<_>>>()?;
    }

    if extract_artifacts {
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

        debug!(
            "extracting {} build artifacts for chal {:?}",
            image_assoc.len(),
            chal.directory
        );

        let assets = image_assoc
            .into_iter()
            .map(|(p, tag)| {
                let name = format!(
                    "asset-container-{}-{}",
                    chal.directory.to_string_lossy().replace("/", "-"),
                    p.from.clone().unwrap()
                );
                let container = docker::create_container(&tag, &name)?;

                let asset_result = extract_asset(p, &container);

                // clean up container even if it failed
                docker::remove_container(&name)?;

                asset_result
            })
            .flatten_ok()
            .collect::<Result<Vec<_>>>()?;

        debug!("Extracted assets: {:?}", assets);
    }
    Ok(built_tags)
}

// /// Push passed tags to registry
// pub fn push_tags(tags: Vec<String>) -> Result<Vec<String>> {
//     let config = get_config()?;

//     let built_tags = tags
//         .iter()
//         .map(|tag| {
//             push_image(tag, &config.registry.build)
//                 .with_context(|| format!("error pushing image {tag}"))
//         })
//         .collect::<Result<_>>()?;

//     Ok(built_tags)
// }

// /// Extract any assets needed from given challenges
// pub fn extract_assets(
//     profile_name: &str,
//     built_chals: Vec<&ChallengeConfig>,
// ) -> Result<Vec<String>> {
//     built_chals.iter().map(|chal| {
//         chal.provide.iter().filter(|p| p.from.is_some()).map(|p| {
//             assets::extract_asset(p, container)
//         })

//         // let tag = format!(
//         //     image_tag!(),
//         //     registry = config.registry.domain,
//         //     challenge = chal.name,
//         //     container = p.name,
//         //     profile = profile_name
//         // );
//     });

//     Ok(vec![])
// }
