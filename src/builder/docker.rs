use anyhow::{anyhow, Context, Error, Result};
use bollard::{image::BuildImageOptions, Docker};
use futures_util::{StreamExt, TryStreamExt};
use simplelog::*;
use std::{io::Read, path::Path};
use tar;
use tempfile::tempfile;
use tokio;

use crate::configparser::challenge::BuildObject;

#[tokio::main(flavor = "current_thread")] // make this a sync function
pub async fn build_image(context: &Path, options: &BuildObject, tag: &str) -> Result<String> {
    trace!("building image in directory {context:?} to tag {tag:?}");
    let client = client()
        .await
        // truncate error chain with new error (returned error is way too verbose)
        .map_err(|_| anyhow!("could not talk to Docker daemon (is DOCKER_HOST correct?)"))?;

    let build_opts = BuildImageOptions {
        dockerfile: options.dockerfile.clone(),
        buildargs: options.args.clone(),
        t: tag.to_string(),
        forcerm: true,
        ..Default::default()
    };

    // tar up image context
    // TODO: dont store the tarball in memory...
    // let mut tar = tar::Builder::new(tempfile()?);
    let mut tar = tar::Builder::new(Vec::new());
    tar.append_dir_all("", context.join(&options.context))
        .with_context(|| "could not create image context tarball")?;
    let tarball = tar.into_inner()?;

    // send to docker daemon
    let mut build_stream = client.build_image(build_opts, None, Some(tarball.into()));

    // stream output to stdout
    while let Some(msg) = build_stream.next().await {
        match msg?.stream {
            Some(log) => info!(
                "building {}: <bright-black>{}</>",
                context.to_string_lossy(),
                log.trim()
            ),
            None => (),
        }
    }

    Ok("".to_string())
}

//
// helper functions
//
pub async fn client() -> Result<Docker> {
    debug!("connecting to docker...");
    let client = Docker::connect_with_defaults()?;
    client.ping().await?;

    Ok(client)
}

#[derive(Debug)]
pub enum EngineType {
    Docker,
    Podman,
}
pub async fn engine_type() -> EngineType {
    let c = client().await.unwrap();
    let version = c.version().await.unwrap();

    if version
        .components
        .unwrap()
        .iter()
        .any(|c| c.name == "Podman Engine")
    {
        EngineType::Podman
    } else {
        EngineType::Docker
    }
}
