// the thing that builds the stuff
// what more is there to say

use anyhow::{anyhow, Context, Error, Result};
use bollard::image::BuildImageOptions;
use futures::stream::{FuturesOrdered, Iter};
use itertools::Itertools;
use std::default;
use std::fmt::Pointer;
use std::iter::zip;
use std::path::{Path, PathBuf};
use tracing::{debug, error, info, trace, warn};

use crate::configparser::challenge::{
    BuildObject, ChallengeConfig, ImageSource::*, Pod, ProvideConfig,
};
use crate::configparser::{enabled_challenges, get_config};
use crate::utils::TryJoinAll;

pub mod artifacts;
pub mod docker;

/// Information about all of a challenge's build artifacts.
#[derive(Debug)]
pub struct BuildResult {
    /// Container image tags of all containers in the challenge, if the challenge has container images.
    /// Will be empty if challenge has no images built from source.
    pub tags: Vec<TagWithSource>,
    /// Path on disk to local assets (both built and static).
    /// Will be empty if challenge has no file assets
    pub assets: Vec<PathBuf>,
}

/// Tag string with added context of where it came from (built locally or an upstream image)
#[derive(Debug, Clone)]
pub enum TagWithSource {
    Upstream(String),
    Built(String),
}

/// Build all enabled challenges for the given profile. Returns tags built
pub async fn build_challenges(
    profile_name: &str,
    push: bool,
    extract_artifacts: bool,
) -> Result<Vec<(&ChallengeConfig, BuildResult)>> {
    enabled_challenges(profile_name)?
        .into_iter()
        .map(|chal| async move {
            build_challenge(profile_name, chal, push, extract_artifacts)
                .await
                .map(|r| (chal, r))
        })
        .try_join_all()
        .await
}

/// Build all images from given challenge, optionally pushing image or extracting artifacts
async fn build_challenge(
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
        .map(|p| async {
            match &p.image_source {
                Image(tag) => Ok(TagWithSource::Upstream(tag.to_string())),
                // build any pods that need building
                Build(build) => {
                    let tag = chal.container_tag_for_pod(profile_name, &p.name)?;

                    let res = docker::build_image(&chal.directory, build, &tag, &p.architecture)
                        .await
                        .with_context(|| {
                            format!(
                                "error building image {} for chal {}",
                                p.name,
                                chal.directory.to_string_lossy()
                            )
                        });
                    // map result tag string into enum
                    res.map(TagWithSource::Built)
                }
            }
        })
        .try_join_all()
        .await?;

    if push {
        // only need to push tags we actually built
        let tags_to_push = built
            .tags
            .iter()
            .filter_map(|t| match t {
                TagWithSource::Built(t) => Some(t),
                TagWithSource::Upstream(_) => None,
            })
            .collect_vec();

        debug!(
            "pushing {} tags for chal {:?}",
            tags_to_push.len(),
            chal.directory
        );

        tags_to_push
            .iter()
            .map(|tag| async move {
                docker::push_image(tag, &config.registry.build)
                    .await
                    .with_context(|| format!("error pushing image {tag}"))
            })
            .try_join_all()
            .await?;
    }

    if extract_artifacts {
        info!("extracting build artifacts for chal {:?}", chal.directory);

        // extract each challenge provide entry
        // this handles both local files and from build containers
        built.assets = chal
            .provide
            .iter()
            .map(|p| async {
                artifacts::extract_asset(chal, p, profile_name)
                    .await
                    .with_context(|| {
                        format!(
                            "failed to extract build artifacts for chal {:?}",
                            chal.directory,
                        )
                    })
            })
            .try_join_all()
            .await?
            // flatten to single vec of all paths
            .into_iter()
            .flatten()
            .collect_vec();

        info!("extracted artifacts: {:?}", built.assets);
    }

    Ok(built)
}
