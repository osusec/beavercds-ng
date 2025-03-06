use std::env;
use std::sync::LazyLock;

use testcontainers::{
    core::{wait::HttpWaitStrategy, IntoContainerPort, WaitFor},
    GenericImage,
};
use testcontainers_modules::{
    cncf_distribution::CncfDistribution,
    k3s::K3s,
    // minio::MinIO,
    testcontainers::{core::ExecCommand, runners::SyncRunner, Container, ImageExt},
};

// use lock to cd only once
static IN_DIR: LazyLock<()> = LazyLock::new(|| env::set_current_dir("./tests/repo").unwrap());
pub fn cd_to_repo() {
    let _ = &*IN_DIR;
}

#[allow(dead_code)] // this is actually included in tests
pub fn registry_ctr() -> Container<CncfDistribution> {
    let registry = CncfDistribution.start().unwrap();

    env::set_var(
        "BEAVERCDS_REGISTRY_DOMAIN",
        format!(
            "{}:{}/testimages",
            registry.get_host().unwrap(),
            registry.get_host_port_ipv4(5000).unwrap()
        ),
    );

    registry
}

#[allow(dead_code)] // this is actually included in tests
pub fn s3_ctr() -> Container<GenericImage> {
    // minio preset does not work with recent image, so make our own from generic
    // let minio = MinIO::default()

    let minio_ready =
        WaitFor::http(HttpWaitStrategy::new("/").with_response_matcher(|r| r.status() == 403));

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

    // set envvars to point at this container
    env::set_var(
        "BEAVERCDS_PROFILES_TESTING_S3_ENDPOINT",
        format!(
            "http://{}:{}",
            minio.get_host().unwrap(),
            minio.get_host_port_ipv4(9000).unwrap()
        ),
    );
    env::set_var("BEAVERCDS_PROFILES_TESTING_S3_REGION", "");
    env::set_var("BEAVERCDS_PROFILES_TESTING_S3_ACCESS_KEY", "testuser");
    env::set_var("BEAVERCDS_PROFILES_TESTING_S3_SECRET_KEY", "notsecure");

    minio
}

#[allow(dead_code)] // this is actually included in tests
pub fn k3s_ctr() -> Container<K3s> {
    let kubeconf_tempdir = tempfile::tempdir().unwrap();
    let k3s_instance = K3s::default()
        .with_conf_mount(&kubeconf_tempdir)
        // .with_privileged(true)
        .with_userns_mode("host")
        .start()
        .unwrap();

    // let kube_port = k3s_instance.get_host_port_ipv4(KUBE_SECURE_PORT);
    // let kube_conf = k3s_instance
    //     .image()
    //     .read_kube_config()
    //     .expect("Cannot read kube conf");

    env::set_var(
        "BEAVERCDS_PROFILES_TESTING_KUBECONFIG",
        kubeconf_tempdir.path(),
    );
    env::set_var("BEAVERCDS_PROFILES_TESTING_KUBECONTEXT", "default");

    k3s_instance
}
