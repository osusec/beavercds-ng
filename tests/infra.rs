use std::{fs::File, io::Write, thread, time::Duration};

use figment::Jail;
use testcontainers::{
    core::{wait::HttpWaitStrategy, IntoContainerPort, Mount, WaitFor},
    GenericImage,
};
use testcontainers_modules::{
    cncf_distribution::CncfDistribution,
    k3s::K3s,
    testcontainers::{core::ExecCommand, runners::SyncRunner, Container, ImageExt},
};

/// Extract bundled test directory into Figment jailoo
#[allow(dead_code)] // this is actually included in tests
pub fn setup_test_repo(j: &Jail) -> Result<(), std::io::Error> {
    TEST_REPO_DIR.extract(j.directory())?;

    Ok(())
}
#[allow(dead_code)] // this is actually included in tests
static TEST_REPO_DIR: include_dir::Dir =
    include_dir::include_dir!("$CARGO_MANIFEST_DIR/tests/repo/");

#[allow(dead_code)] // this is actually included in tests
pub fn registry_ctr(j: &mut Jail) -> Container<CncfDistribution> {
    let registry = CncfDistribution.start().unwrap();

    let vars = [(
        "BEAVERCDS_REGISTRY_DOMAIN",
        format!(
            "{}:{}/testimages",
            registry.get_host().unwrap(),
            registry.get_host_port_ipv4(5000).unwrap()
        ),
    )];

    for (k, v) in vars {
        j.set_env(k, v);
    }

    registry
}

#[allow(dead_code)] // this is actually included in tests
pub fn s3_ctr(j: &mut Jail) -> Container<GenericImage> {
    // minio preset does not work with recent image, so make our own from generic
    // let minio = MinIO::default()

    let minio_ready = WaitFor::http(
        HttpWaitStrategy::new("/minio/health/live").with_expected_status_code(200_u16),
    );

    let minio = GenericImage::new("quay.io/minio/minio", "latest")
        .with_exposed_port(9000.tcp())
        .with_wait_for(minio_ready.clone())
        .with_cmd(["server", "/data"])
        .with_env_var("MINIO_ROOT_USER", "testuser")
        .with_env_var("MINIO_ROOT_PASSWORD", "notsecure")
        .start()
        .unwrap();

    minio
        .exec(
            // create bucket and permissions
            ExecCommand::new([
                "/bin/sh",
                "-xec",
                "
                    /usr/bin/mc alias set self http://localhost:9000 testuser notsecure;
                    /usr/bin/mc mb self/testbucket;
                    /usr/bin/mc anonymous set download self/testbucket;
                ",
            ])
            .with_container_ready_conditions(vec![minio_ready]),
        )
        .unwrap();

    // some sort of race condition in the bucket creation
    // give exec above a bit of time to apply
    thread::sleep(Duration::from_secs(1));

    // set envvars to point at this container
    j.set_env(
        "BEAVERCDS_PROFILES_TESTING_S3_ENDPOINT",
        format!(
            "http://{}:{}",
            minio.get_host().unwrap(),
            minio.get_host_port_ipv4(9000).unwrap()
        ),
    );
    j.set_env("BEAVERCDS_PROFILES_TESTING_S3_REGION", "");
    j.set_env("BEAVERCDS_PROFILES_TESTING_S3_ACCESS_KEY", "testuser");
    j.set_env("BEAVERCDS_PROFILES_TESTING_S3_SECRET_KEY", "notsecure");

    minio
}

#[allow(dead_code)] // this is actually included in tests
pub fn k3s_ctr(j: &mut Jail) -> Container<K3s> {
    let kcfg_tempdir = tempfile::tempdir().unwrap();

    // The upstream testcontainer for k3s does not work rootless, so define it
    // manually with the needed flag added on rootless systems
    //
    let k3s_instance = K3s::default()
        .with_conf_mount(&kcfg_tempdir)
        .with_privileged(true)
        // .with_cmd(["server", "--disable=traefik@server:*", "--rootless"])
        .start()
        .unwrap();

    // let kube_port = k3s_instance.get_host_port_ipv4(KUBE_SECURE_PORT);

    // write out kubeconfig
    let kube_conf = k3s_instance
        .image()
        .read_kube_config()
        .expect("Cannot read kube conf");

    j.create_file("test.kubeconfig", &kube_conf)
        .expect("could not write kubeconfig in jail");

    // // nested func for tokio::main reasons
    // #[tokio::main(flavor = "current_thread")] // make this a sync function
    // async fn k3s_command() -> Vec<&'static str> {
    //     // we have a helper for this! podman almost always means rootless
    //     match beavercds_ng::clients::engine_type().await {
    //         beavercds_ng::clients::EngineType::Docker => vec!["server"],
    //         beavercds_ng::clients::EngineType::Podman => vec![
    //             "server",
    //             "--kubelet-arg=feature-gates=KubeletInUserNamespace=true",
    //         ],
    //     }
    // }

    // // TODO: pin to specific k8s version tag?
    // let k3s_instance = GenericImage::new("docker.io/rancher/k3s", "latest")
    //     .with_wait_for(WaitFor::message_on_stderr(
    //         "Node controller sync successful",
    //     ))
    //     .with_mapped_port(6443, 6443.tcp())
    //     .with_mapped_port(8000, 80.tcp())
    //     .with_mapped_port(8443, 443.tcp())
    //     // mount tempdir in for generated kubeconfig
    //     .with_mount(Mount::bind_mount(
    //         kubeconf_tempdir.path().to_str().unwrap(),
    //         "/output",
    //     ))
    //     .with_env_var("K3S_TOKEN", "notsecure")
    //     .with_env_var("K3S_KUBECONFIG_OUTPUT", "/output/kubeconfig.yaml")
    //     .with_env_var("K3S_KUBECONFIG_MODE", "666")
    //     .with_cmd(k3s_command().iter().map(<_>::to_string))
    //     .start()
    //     .unwrap();

    j.set_env("BEAVERCDS_PROFILES_TESTING_KUBECONFIG", "test.kubeconfig");
    j.set_env("BEAVERCDS_PROFILES_TESTING_KUBECONTEXT", "default");

    k3s_instance
}
