use figment::Jail;
use std::collections::HashMap;
use std::fmt::Display;

#[cfg(test)]
use pretty_assertions::{assert_eq, assert_ne};

use crate::configparser::config::*;

#[test]
/// Test parsing RCDS config where all fields are specified in the yaml
fn all_yaml() {
    figment::Jail::expect_with(|jail| {
        jail.clear_env();
        jail.create_file(
            "rcds.yaml",
            r#"
                flag_regex: test{[a-zA-Z_]+}

                registry:
                    domain: registry.example/test
                    build:
                        user: admin
                        pass: notrealcreds
                    cluster:
                        user: cluster
                        pass: alsofake

                defaults:
                    difficulty: 1
                    resources: { cpu: 1, memory: 500M }

                points:
                  - difficulty: 1
                    min: 0
                    max: 1337

                deploy:
                    testing:
                        misc/foo: true
                        web/bar: false

                profiles:
                    testing:
                        frontend_url: https://frontend.example
                        frontend_token: secretsecretsecret
                        challenges_domain: chals.frontend.example
                        kubecontext: testcluster
                        s3:
                            bucket_name: asset_testing
                            endpoint: s3.example
                            region: us-fake-1
                            access_key: accesskey
                            secret_key: secretkey
                        dns:
                            provider: somebody
                            thing: whatever
            "#,
        )?;

        let config = match parse() {
            Ok(c) => Ok(c),
            // figment::Error cannot coerce from anyhow::Error natively
            Err(e) => Err(figment::Error::from(format!("{:?}", e))),
        }?;

        let expected = RcdsConfig {
            flag_regex: "test{[a-zA-Z_]+}".to_string(),
            registry: Registry {
                domain: "registry.example/test".to_string(),
                build: UserPass {
                    user: "admin".to_string(),
                    pass: "notrealcreds".to_string(),
                },
                cluster: UserPass {
                    user: "cluster".to_string(),
                    pass: "alsofake".to_string(),
                },
            },
            defaults: Defaults {
                difficulty: 1,
                resources: Resource {
                    cpu: 1,
                    memory: "500M".to_string(),
                },
            },
            points: vec![ChallengePoints {
                difficulty: 1,
                min: 0,
                max: 1337,
            }],

            deploy: HashMap::from([(
                "testing".to_string(),
                ProfileDeploy {
                    challenges: HashMap::from([
                        ("web/bar".to_string(), false),
                        ("misc/foo".to_string(), true),
                    ]),
                },
            )]),
            profiles: HashMap::from([(
                "testing".to_string(),
                ProfileConfig {
                    frontend_url: "https://frontend.example".to_string(),
                    frontend_token: "secretsecretsecret".to_string(),
                    challenges_domain: "chals.frontend.example".to_string(),
                    kubeconfig: None,
                    kubecontext: "testcluster".to_string(),
                    s3: S3Config {
                        bucket_name: "asset_testing".to_string(),
                        endpoint: "s3.example".to_string(),
                        region: "us-fake-1".to_string(),
                        access_key: "accesskey".to_string(),
                        secret_key: "secretkey".to_string(),
                    },
                    dns: serde_yml::to_value(HashMap::from([
                        ("provider", "somebody"),
                        ("thing", "whatever"),
                    ]))
                    .unwrap(),
                },
            )]),
        };

        assert_eq!(config, expected);

        Ok(())
    });
}

#[test]
/// Test parsing RCDS config where some secrets are overridden by envvars
fn yaml_with_env_overrides() {
    figment::Jail::expect_with(|jail| {
        jail.clear_env();
        jail.create_file(
            "rcds.yaml",
            r#"
                flag_regex: test{[a-zA-Z_]+}

                registry:
                    domain: registry.example/test
                    build:
                        user: admin
                        pass: notrealcreds
                    cluster:
                        user: cluster
                        pass: alsofake

                defaults:
                    difficulty: 1
                    resources: { cpu: 1, memory: 500M }

                points:
                  - difficulty: 1
                    min: 0
                    max: 1337

                deploy:
                    testing:
                        misc/foo: true
                        web/bar: false

                profiles:
                    testing:
                        frontend_url: https://frontend.example
                        frontend_token: secretsecretsecret
                        challenges_domain: chals.frontend.example
                        kubecontext: testcluster
                        s3:
                            bucket_name: asset_testing
                            endpoint: s3.example
                            region: us-fake-1
                            access_key: accesskey
                            secret_key: secretkey
                        dns:
                            provider: somebody
                            thing: whatever
            "#,
        )?;

        jail.set_env("BEAVERCDS_REGISTRY_BUILD_USER", "envbuilduser");
        jail.set_env("BEAVERCDS_REGISTRY_BUILD_PASS", "envbuildpass");

        jail.set_env("BEAVERCDS_REGISTRY_CLUSTER_USER", "envclusteruser");
        jail.set_env("BEAVERCDS_REGISTRY_CLUSTER_PASS", "envclusterpass");

        jail.set_env("BEAVERCDS_PROFILES_TESTING_FRONTEND_TOKEN", "envtoken");
        jail.set_env("BEAVERCDS_PROFILES_TESTING_S3_ACCESS_KEY", "envkey");
        jail.set_env("BEAVERCDS_PROFILES_TESTING_S3_SECRET_KEY", "envsecret");

        let config = match parse() {
            Err(e) => Err(figment::Error::from(format!("{:?}", e))),
            Ok(config) => Ok(config),
        }?;

        // also check that the envvar overrides were applied
        assert_eq!(config.registry.build.user, "envbuilduser");
        assert_eq!(config.registry.build.pass, "envbuildpass");
        assert_eq!(config.registry.cluster.user, "envclusteruser");
        assert_eq!(config.registry.cluster.pass, "envclusterpass");

        let profile = config.profiles.get("testing").unwrap();

        assert_eq!(profile.frontend_token, "envtoken");
        assert_eq!(profile.s3.access_key, "envkey");
        assert_eq!(profile.s3.secret_key, "envsecret");

        Ok(())
    });
}

#[test]
/// Test parsing RCDS config where secrets are set in envvars and omitted from yaml
fn partial_yaml_with_env() {
    figment::Jail::expect_with(|jail| {
        jail.clear_env();
        jail.create_file(
            "rcds.yaml",
            r#"
                flag_regex: test{[a-zA-Z_]+}

                registry:
                    domain: registry.example/test

                defaults:
                    difficulty: 1
                    resources: { cpu: 1, memory: 500M }

                points:
                  - difficulty: 1
                    min: 0
                    max: 1337

                deploy:
                    testing:
                        misc/foo: true
                        web/bar: false

                profiles:
                    testing:
                        frontend_url: https://frontend.example
                        challenges_domain: chals.frontend.example
                        kubecontext: testcluster
                        s3:
                            bucket_name: asset_testing
                            endpoint: s3.example
                            region: us-fake-1
                        dns:
                            provider: somebody
                            thing: whatever
            "#,
        )?;

        jail.set_env("BEAVERCDS_REGISTRY_BUILD_USER", "envbuilduser");
        jail.set_env("BEAVERCDS_REGISTRY_BUILD_PASS", "envbuildpass");

        jail.set_env("BEAVERCDS_REGISTRY_CLUSTER_USER", "envclusteruser");
        jail.set_env("BEAVERCDS_REGISTRY_CLUSTER_PASS", "envclusterpass");

        jail.set_env("BEAVERCDS_PROFILES_TESTING_FRONTEND_TOKEN", "envtoken");
        jail.set_env("BEAVERCDS_PROFILES_TESTING_S3_ACCESS_KEY", "envkey");
        jail.set_env("BEAVERCDS_PROFILES_TESTING_S3_SECRET_KEY", "envsecret");

        let config = match parse() {
            Err(e) => Err(figment::Error::from(format!("{:?}", e))),
            Ok(config) => Ok(config),
        }?;

        // also check that the envvar overrides were applied
        assert_eq!(config.registry.build.user, "envbuilduser");
        assert_eq!(config.registry.build.pass, "envbuildpass");
        assert_eq!(config.registry.cluster.user, "envclusteruser");
        assert_eq!(config.registry.cluster.pass, "envclusterpass");

        let profile = config.profiles.get("testing").unwrap();

        assert_eq!(profile.frontend_token, "envtoken");
        assert_eq!(profile.s3.access_key, "envkey");
        assert_eq!(profile.s3.secret_key, "envsecret");

        Ok(())
    });
}

#[test]
/// Test attempting to parse missing config file
fn bad_no_file() {
    figment::Jail::expect_with(|jail| {
        jail.clear_env();
        // don't create file

        let config = parse();
        assert!(config.is_err());

        Ok(())
    });
}

#[test]
/// Test empty config file
fn bad_empty_file() {
    figment::Jail::expect_with(|jail| {
        jail.create_file("rcds.yaml", "")?;

        let config = parse();
        assert!(config.is_err());

        Ok(())
    });
}

#[test]
/// Test parsing yaml that is missing some fields
fn bad_yaml_missing_secrets() {
    figment::Jail::expect_with(|jail| {
        jail.create_file(
            "rcds.yaml",
            r#"
                flag_regex: test{[a-zA-Z_]+}

                registry:
                    domain: registry.example/test

                defaults:
                    difficulty: 1
                    resources: { cpu: 1, memory: 500M }

                points:
                  - difficulty: 1
                    min: 0
                    max: 1337

                deploy:
                    testing:
                        misc/foo: true
                        web/bar: false

                profiles:
                    testing:
                        frontend_url: https://frontend.example
                        challenges_domain: chals.frontend.example
                        kubecontext: testcluster
                        s3:
                            bucket_name: asset_testing
                            endpoint: s3.example
                            region: us-fake-1
                        dns:
                            provider: somebody
                            thing: whatever
            "#,
        )?;

        let config = parse();
        assert!(config.is_err());

        Ok(())
    });
}
