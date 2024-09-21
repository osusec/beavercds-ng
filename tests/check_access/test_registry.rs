use anyhow::{Error, Result};
use beavercds_ng::{self, configparser::config::*};
use cmd_lib::run_cmd;
use common::empty_config;

// crate:: or super:: doesnt see this from in a subfolder
#[path = "../common.rs"]
mod common;

//
// Integration tests with real registry
//

/// Set up local registry container to test against, return "domain:port"
fn registry_setup(creds: &UserPass) {
    // shell out to docker to create registry
    // overwrite entrypoint to set up credentials first
    let (user, pass) = (&creds.user, &creds.pass);
    run_cmd!(
        docker run
        -d
        --publish="5000:5000"
        --name="beavercds_test_registry"
        --entrypoint=""
        --env="REGISTRY_AUTH=htpasswd"
        --env="REGISTRY_AUTH_HTPASSWD_PATH=/auth/htpasswd"
        --env="REGISTRY_AUTH_HTPASSWD_REALM=Registry Realm"
        registry:2
        sh -c "htpasswd -Bbs $user $pass > /auth/htpasswd && registry serve"
    )
    .unwrap();
}

/// clean up registry container after tests (if running)
fn registry_teardown() {
    // run_cmd!(docker rm -f beavercds_test_registry).unwrap();
    run_cmd!(
        info "waiting...";
        sleep 5;
        docker rm -f beavercds_test_registry)
    .unwrap();
}

/// Config is OK and registry behaves as normal
#[test]
fn no_problems() {
    let creds = UserPass {
        user: "user".to_string(),
        pass: "pass".to_string(),
    };

    registry_setup(&creds);

    // generate fake config with credentials
    beavercds_ng::configparser::CONFIG
        .set(RcdsConfig {
            registry: Registry {
                domain: "localhost:5000".to_string(),
                build: creds.clone(),
                cluster: creds.clone(),
            },
            ..empty_config()
        })
        .unwrap();

    // run check-access --registry against test registry
    let check_result = beavercds_ng::access_handlers::docker::check("default");

    registry_teardown();

    assert!(check_result.is_ok())
}

/// Invalid credentials
#[test]
fn bad_credentials() {
    let creds = UserPass {
        user: "user".to_string(),
        pass: "pass".to_string(),
    };

    let bad_creds = UserPass {
        user: "bogus".to_string(),
        pass: "hunter2".to_string(),
    };

    registry_setup(&creds);

    // generate fake config with credentials
    beavercds_ng::configparser::CONFIG
        .set(RcdsConfig {
            registry: Registry {
                domain: "localhost:5000".to_string(),
                build: bad_creds.clone(),
                cluster: bad_creds.clone(),
            },
            ..empty_config()
        })
        .unwrap();

    // run check-access --registry against test registry with bad creds
    let check_result = beavercds_ng::access_handlers::docker::check("default");

    registry_teardown();

    assert!(check_result.is_err())
}

/// Registry is unreachable
#[test]
fn bad_registry() {
    let creds = UserPass {
        user: "user".to_string(),
        pass: "pass".to_string(),
    };

    // *don't* set up the registry this time

    // generate fake config with credentials
    beavercds_ng::configparser::CONFIG
        .set(RcdsConfig {
            registry: Registry {
                domain: "localhost:5000".to_string(),
                build: creds.clone(),
                cluster: creds.clone(),
            },
            ..empty_config()
        })
        .unwrap();

    // run check-access --registry against non-existent registry
    let check_result = beavercds_ng::access_handlers::docker::check("default");

    registry_teardown();

    assert!(check_result.is_err())
}
