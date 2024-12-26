use anyhow::{anyhow, bail, Context, Error, Result};
use bollard::auth::DockerCredentials;
use bollard::container::{
    Config, CreateContainerOptions, DownloadFromContainerOptions, RemoveContainerOptions,
};
use bollard::errors::Error as DockerError;
use bollard::image::{BuildImageOptions, PushImageOptions};
use bollard::Docker;
use core::fmt;
use futures::{StreamExt, TryStreamExt};
use simplelog::*;
use std::fs::File;
use std::io::{Seek, Write};
use std::sync::LazyLock;
use std::{fs, io};
use std::{io::Read, path::Path};
use tar;
use tempfile::{tempdir_in, tempfile};
use tokio;

use crate::configparser::challenge::BuildObject;
use crate::configparser::UserPass;

#[tokio::main(flavor = "current_thread")] // make this a sync function
pub async fn build_image(context: &Path, options: &BuildObject, tag: &str) -> Result<String> {
    trace!("building image in directory {context:?} to tag {tag:?}");
    let client = client().await?;

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
    while let Some(item) = build_stream.next().await {
        match item {
            // error from stream?
            Err(e) => match e {
                DockerError::DockerStreamError { error } => bail!("build error: {error}"),
                other => bail!("build error: {other:?}"),
            },
            Ok(msg) => {
                // error from daemon?
                if let Some(e) = msg.error_detail {
                    bail!(
                        "error building image: {}",
                        e.message.unwrap_or("".to_string())
                    )
                }

                if let Some(log) = msg.stream {
                    info!(
                        "building {}: <bright-black>{}</>",
                        context.to_string_lossy(),
                        // tag,
                        log.trim()
                    )
                }
            }
        }
    }

    Ok(tag.to_string())
}

#[tokio::main(flavor = "current_thread")] // make this a sync function
pub async fn push_image(image_tag: &str, creds: &UserPass) -> Result<String> {
    info!("pushing image {image_tag:?} to registry");
    let client = client().await?;

    let (image, tag) = image_tag
        .rsplit_once(":")
        .context("failed to get tag from full image string")?;

    let opts = PushImageOptions { tag };
    let creds = DockerCredentials {
        username: Some(creds.user.clone()),
        password: Some(creds.pass.clone()),
        ..Default::default()
    };

    let mut push_stream = client.push_image(image, Some(opts), Some(creds));

    // stream output to stdout
    while let Some(item) = push_stream.next().await {
        match item {
            // error from stream?
            Err(DockerError::DockerResponseServerError {
                status_code,
                message,
            }) => bail!("error from daemon: {message}"),
            Err(e) => bail!("{e:?}"),
            Ok(msg) => {
                debug!("{msg:?}");
                if let Some(progress) = msg.progress_detail {
                    info!("progress: {:?}/{:?}", progress.current, progress.total);
                }
            }
        }
    }
    Ok(tag.to_string())
}

#[tokio::main(flavor = "current_thread")] // make this a sync function
pub async fn create_container(image_tag: &str, name: &str) -> Result<String> {
    debug!("creating container {name:?} from image {image_tag:?}");
    let client = client().await?;

    let opts = CreateContainerOptions {
        name: name.to_string(),
        ..Default::default()
    };
    let config = Config {
        image: Some(image_tag),
        ..Default::default()
    };

    let container = client.create_container(Some(opts), config).await?;
    Ok(container.id)
}

#[tokio::main(flavor = "current_thread")] // make this a sync function
pub async fn remove_container(name: &str) -> Result<()> {
    debug!("removing container {name:?}");
    let client = client().await?;

    let opts = RemoveContainerOptions {
        force: true,
        ..Default::default()
    };
    client.remove_container(name, Some(opts)).await?;

    Ok(())
}

pub async fn copy_file(
    container_id: &str,
    from_path: &str,
    rename_to: Option<&str>,
) -> Result<String> {
    let client = client().await?;

    // if no rename is given, use basename of `from` as target path
    let target_path = match rename_to {
        Some(to) => to,
        None => Path::new(from_path).file_name().unwrap().to_str().unwrap(),
    };

    info!("copying {container_id}:{from_path} to {target_path}");

    // Download single file from container in an archive
    let opts = DownloadFromContainerOptions { path: from_path };
    let mut dl_stream = client.download_from_container(container_id, Some(opts));

    // scratch dir in chal repo (two vars for scoping reasons)
    // let mut tempdir_full = tempdir_in(".")?;
    // let tempdir = tempdir_full.path();

    fs::create_dir("./.tempdir");
    let tempdir = Path::new("./.tempdir");

    // collect byte stream chunks into full file
    let mut tarfile = File::create(tempdir.join(format!("download_{target_path}.tar")))?;
    while let Some(chunk) = dl_stream.next().await {
        tarfile.write_all(&chunk?)?;
    }
    tarfile.rewind();

    // unpack file retrieved to temp dir
    trace!("extracting download tar to {:?}", tempdir);
    let mut tar = tar::Archive::new(tarfile);

    // extract single file from archive to disk
    // we only copied out one file, so this tar should only have one file
    if let Some(Ok(mut entry)) = tar.entries()?.next() {
        let mut target = File::create_new(target_path)?;
        io::copy(&mut entry, &mut target);
    } else {
        bail!("downloaded archive for {container_id}:{from_path} has no files in it!");
    }

    Ok(target_path.to_string())
}

//
// helper functions
//

// connect to Docker/Podman daemon once and share client
static CLIENT: LazyLock<std::result::Result<Docker, bollard::errors::Error>> =
    LazyLock::new(|| {
        debug!("connecting to docker...");
        Docker::connect_with_defaults()
    });
pub async fn client() -> Result<Docker> {
    let c = CLIENT
        .as_ref()
        .map_err(|_| anyhow!("could not talk to Docker daemon (is DOCKER_HOST correct?)"))?;
    c.ping().await?;

    Ok(c.clone())
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
