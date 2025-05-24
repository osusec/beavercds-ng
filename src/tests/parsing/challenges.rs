use figment::Jail;
use std::collections::HashMap;
use std::path::PathBuf;

#[cfg(test)]
use pretty_assertions::{assert_eq, assert_ne};

use crate::configparser::{challenge::*, config::Resource};

const VALID_CONFIG: &str = r#"
    flag_regex: ctf{[a-zA-Z_\-0-9]*}

    defaults:
        difficulty: 1
        resources: { cpu: 1, memory: 200Mi }

    points:
      - difficulty: 1
        min: 0
        max: 1337

    registry:
        domain: images.example.ctf
        build: { user: "", pass: "" }
        cluster: { user: "", pass: "" }
    deploy: {}
    profiles: {}

"#;

const VALID_CHAL: &str = r#"
    name: testchal
    author: nobody
    description: just a test challenge
    difficulty: 1

    flag:
        text: test{it-works}

    provide: []
    pods: []
"#;

#[test]
/// No challenge files should parse correctly
fn no_challenges() {
    figment::Jail::expect_with(|jail| {
        jail.create_file("rcds.yaml", VALID_CONFIG)?;
        let chals = parse_all();

        assert!(chals.is_ok());
        assert_eq!(chals.unwrap().len(), 0);

        Ok(())
    })
}

#[test]
/// Challenge yaml at repo root should not parse
fn challenge_in_root() {
    figment::Jail::expect_with(|jail| {
        jail.create_file("rcds.yaml", VALID_CONFIG)?;
        jail.create_file("challenge.yaml", "name: test")?;

        let chals = parse_all();

        assert!(chals.is_ok());
        assert_eq!(chals.unwrap().len(), 0);

        Ok(())
    })
}

#[test]
/// Challenge yaml one folder down should not parse
fn challenge_one_level() {
    figment::Jail::expect_with(|jail| {
        jail.create_file("rcds.yaml", VALID_CONFIG)?;
        let dir = jail.create_dir("foo")?;
        jail.create_file(dir.join("challenge.yaml"), "name: test")?;

        let chals = parse_all();

        assert!(chals.is_ok());
        assert_eq!(chals.unwrap().len(), 0);

        Ok(())
    })
}

#[test]
/// Challenge yaml two folders down should be parsed
fn challenge_two_levels() {
    figment::Jail::expect_with(|jail| {
        jail.create_file("rcds.yaml", VALID_CONFIG)?;
        let dir = jail.create_dir("foo/test")?;
        jail.create_file(dir.join("challenge.yaml"), VALID_CHAL)?;

        let chals = parse_all();

        assert!(chals.is_ok());
        let chals = chals.unwrap();
        assert_eq!(chals.len(), 1);

        assert_eq!(
            chals[0],
            ChallengeConfig {
                name: "testchal".to_string(),
                author: "nobody".to_string(),
                description: "just a test challenge".to_string(),
                difficulty: 1,

                category: "foo".to_string(),
                directory: PathBuf::from("foo/test"),

                flag: FlagType::String {
                    string: "test{it-works}".to_string()
                },

                provide: vec![],
                pods: vec![],
            }
        );

        Ok(())
    })
}

#[test]
/// Challenge yaml three folders down should not parsed
fn challenge_three_levels() {
    figment::Jail::expect_with(|jail| {
        jail.create_file("rcds.yaml", VALID_CONFIG)?;
        let dir = jail.create_dir("chals/foo/test")?;
        jail.create_file(dir.join("challenge.yaml"), VALID_CHAL)?;

        let chals = parse_all();

        assert!(chals.is_ok());
        assert_eq!(chals.unwrap().len(), 0);

        Ok(())
    })
}

#[test]
fn challenge_missing_fields() {
    figment::Jail::expect_with(|jail| {
        let dir = jail.create_dir("test/noflag")?;
        jail.create_file("rcds.yaml", VALID_CONFIG)?;
        jail.create_file(
            dir.join("challenge.yaml"),
            r#"
            name: testchal
            author: nobody
            description: just a test challenge
            difficulty: 1
        "#,
        )?;

        let dir = jail.create_dir("test/noauthor")?;
        jail.create_file(
            dir.join("challenge.yaml"),
            r#"
            name: testchal
            description: just a test challenge
            difficulty: 1

            flag:
                text: test{asdf}
        "#,
        )?;

        let dir = jail.create_dir("test/nodescrip")?;
        jail.create_file(
            dir.join("challenge.yaml"),
            r#"
            name: testchal
            author: nobody
            difficulty: 1

            flag:
                text: test{asdf}
        "#,
        )?;

        let chals = parse_all();
        assert!(chals.is_err());
        let errs = chals.unwrap_err();

        assert_eq!(errs.len(), 3);

        Ok(())
    })
}

#[test]
/// Challenges can omit both provides and pods fields if needed
fn challenge_no_provides_or_pods() {
    figment::Jail::expect_with(|jail| {
        jail.create_file("rcds.yaml", VALID_CONFIG)?;
        let dir = jail.create_dir("foo/test")?;
        jail.create_file(
            dir.join("challenge.yaml"),
            r#"
            name: testchal
            author: nobody
            description: just a test challenge
            difficulty: 1

            flag:
                text: test{it-works}
        "#,
        )?;

        let chals = parse_all().unwrap();

        assert_eq!(chals[0].provide, vec![] as Vec<ProvideConfig>);
        assert_eq!(chals[0].pods, vec![] as Vec<Pod>);

        Ok(())
    })
}

#[test]
/// Challenge provide files parse correctly
fn challenge_provide() {
    figment::Jail::expect_with(|jail| {
        jail.create_file("rcds.yaml", VALID_CONFIG)?;
        let dir = jail.create_dir("foo/test")?;
        jail.create_file(
            dir.join("challenge.yaml"),
            r#"
            name: testchal
            author: nobody
            description: just a test challenge
            difficulty: 1

            flag:
                text: test{it-works}

            provide:
                - foo.txt

                - include: foo2.txt

                - include:
                    - bar.txt
                    - baz.txt

                - as: oranges
                  include: apples

                - as: stuff.zip
                  include:
                    - ducks
                    - beavers

                - from: container
                  include:
                    - /foo/bar

                - from: container
                  as: pears
                  include: /usr/lib/peaches

                - from: container
                  as: shells.zip
                  include:
                    - /usr/bin/bash
                    - /usr/bin/zsh
        "#,
        )?;

        let chals = parse_all().unwrap();

        assert_eq!(
            chals[0].provide,
            vec![
                ProvideConfig::FromRepoSingle {
                    file: "foo.txt".into()
                },
                ProvideConfig::FromRepoSingle {
                    file: "foo2.txt".into()
                },
                ProvideConfig::FromRepo {
                    files: vec!["bar.txt".into(), "baz.txt".into()]
                },
                ProvideConfig::FromRepoRename {
                    from: "apples".into(),
                    to: "oranges".into()
                },
                ProvideConfig::FromRepoArchive {
                    files: vec!["ducks".into(), "beavers".into()],
                    archive_name: "stuff.zip".into()
                },
                ProvideConfig::FromContainer {
                    container: "container".to_string(),
                    files: vec!["/foo/bar".into()]
                },
                ProvideConfig::FromContainerRename {
                    container: "container".to_string(),
                    from: "/usr/lib/peaches".into(),
                    to: "pears".into(),
                },
                ProvideConfig::FromContainerArchive {
                    container: "container".to_string(),
                    files: vec!["/usr/bin/bash".into(), "/usr/bin/zsh".into()],
                    archive_name: "shells.zip".into(),
                }
            ],
        );

        Ok(())
    })
}

#[test]
/// Challenge provide files dont parse if include is missing from object form
fn challenge_provide_no_include() {
    figment::Jail::expect_with(|jail| {
        jail.create_file("rcds.yaml", VALID_CONFIG)?;
        let dir = jail.create_dir("foo/test")?;
        jail.create_file(
            dir.join("challenge.yaml"),
            r#"
            name: testchal
            author: nobody
            description: just a test challenge
            difficulty: 1

            flag:
                text: test{it-works}

            provide:
                - as: bad.zip
        "#,
        )?;

        let chals = parse_all();
        assert!(chals.is_err());
        let errs = chals.unwrap_err();
        assert_eq!(errs.len(), 1);

        Ok(())
    })
}

#[test]
/// Challenges should be able to have multiple pods
fn challenge_pods() {
    figment::Jail::expect_with(|jail| {
        jail.create_file("rcds.yaml", VALID_CONFIG)?;
        let dir = jail.create_dir("foo/test")?;
        jail.create_file(
            dir.join("challenge.yaml"),
            r#"
            name: testchal
            author: nobody
            description: just a test challenge
            difficulty: 1

            flag:
                text: test{it-works}

            pods:
                - name: foo
                  image: nginx
                  replicas: 2
                  ports:
                    - internal: 80
                      expose:
                        http: test.chals.example.com

                - name: bar
                  build: .
                  replicas: 1
                  ports:
                    - internal: 8000
                      expose:
                        tcp: 12345
        "#,
        )?;

        let chals = parse_all().unwrap();

        assert_eq!(
            chals[0].pods,
            vec![
                Pod {
                    name: "foo".to_string(),
                    image_source: ImageSource::Image("nginx".to_string()),
                    replicas: 2,
                    env: ListOrMap::Map(HashMap::new()),
                    resources: Some(Resource {
                        cpu: 1,
                        memory: "200Mi".to_string()
                    }),
                    architecture: "amd64".to_string(),
                    ports: vec![PortConfig {
                        internal: 80,
                        expose: ExposeType::Http("test.chals.example.com".to_string())
                    }],
                    volume: None
                },
                Pod {
                    name: "bar".to_string(),
                    image_source: ImageSource::Build(BuildObject {
                        context: ".".to_string(),
                        dockerfile: "Dockerfile".to_string(),
                        args: HashMap::new()
                    }),
                    replicas: 1,
                    env: ListOrMap::Map(HashMap::new()),
                    resources: Some(Resource {
                        cpu: 1,
                        memory: "200Mi".to_string()
                    }),
                    architecture: "amd64".to_string(),
                    ports: vec![PortConfig {
                        internal: 8000,
                        expose: ExposeType::Tcp(12345)
                    }],
                    volume: None
                },
            ]
        );

        Ok(())
    })
}

#[test]
/// Challenge pods can use simple or complex build options
fn challenge_pod_build() {
    figment::Jail::expect_with(|jail| {
        jail.create_file("rcds.yaml", VALID_CONFIG)?;
        let dir = jail.create_dir("foo/test")?;
        jail.create_file(
            dir.join("challenge.yaml"),
            r#"
            name: testchal
            author: nobody
            description: just a test challenge
            difficulty: 1

            flag:
                text: test{it-works}

            pods:
                - name: foo
                  build: .
                  replicas: 1
                  ports:
                    - internal: 80
                      expose:
                        http: test.chals.example.com

                - name: bar
                  build:
                    context: image/
                    dockerfile: Containerfile
                    args:
                      FOO: this
                      BAR: that
                  replicas: 1
                  ports:
                    - internal: 80
                      expose:
                        http: test2.chals.example.com
        "#,
        )?;

        let chals = parse_all().unwrap();

        assert_eq!(
            chals[0].pods,
            vec![
                Pod {
                    name: "foo".to_string(),

                    image_source: ImageSource::Build(BuildObject {
                        context: ".".to_string(),
                        dockerfile: "Dockerfile".to_string(),
                        args: HashMap::new()
                    }),
                    replicas: 1,
                    env: ListOrMap::Map(HashMap::new()),
                    resources: Some(Resource {
                        cpu: 1,
                        memory: "200Mi".to_string()
                    }),
                    architecture: "amd64".to_string(),
                    ports: vec![PortConfig {
                        internal: 80,
                        expose: ExposeType::Http("test.chals.example.com".to_string())
                    }],
                    volume: None
                },
                Pod {
                    name: "bar".to_string(),
                    image_source: ImageSource::Build(BuildObject {
                        context: "image/".to_string(),
                        dockerfile: "Containerfile".to_string(),
                        args: HashMap::from([
                            ("FOO".to_string(), "this".to_string()),
                            ("BAR".to_string(), "that".to_string()),
                        ])
                    }),
                    replicas: 1,
                    env: ListOrMap::Map(HashMap::new()),
                    resources: Some(Resource {
                        cpu: 1,
                        memory: "200Mi".to_string()
                    }),
                    architecture: "amd64".to_string(),
                    ports: vec![PortConfig {
                        internal: 80,
                        expose: ExposeType::Http("test2.chals.example.com".to_string())
                    }],
                    volume: None
                }
            ]
        );

        Ok(())
    })
}

#[test]
/// Challenge pod envvars can be set as either string list or map
fn challenge_pod_env() {
    figment::Jail::expect_with(|jail| {
        jail.create_file("rcds.yaml", VALID_CONFIG)?;
        let dir = jail.create_dir("foo/test")?;
        jail.create_file(
            dir.join("challenge.yaml"),
            r#"
            name: testchal
            author: nobody
            description: just a test challenge
            difficulty: 1

            flag:
                text: test{it-works}

            pods:
                - name: foo
                  image: nginx
                  env:
                    FOO: this
                    BAR: that
                  replicas: 1
                  ports:
                    - internal: 80
                      expose:
                        http: test.chals.example.com

                - name: bar
                  image: nginx
                  env:
                    - FOO=this
                    - BAR=that
                  replicas: 1
                  ports:
                    - internal: 80
                      expose:
                        http: test2.chals.example.com
        "#,
        )?;

        let chals = parse_all().unwrap();

        assert_eq!(
            chals[0].pods,
            vec![
                Pod {
                    name: "foo".to_string(),

                    image_source: ImageSource::Image("nginx".to_string()),
                    replicas: 1,
                    env: ListOrMap::Map(HashMap::from([
                        ("FOO".to_string(), "this".to_string()),
                        ("BAR".to_string(), "that".to_string()),
                    ])),
                    resources: Some(Resource {
                        cpu: 1,
                        memory: "200Mi".to_string()
                    }),
                    architecture: "amd64".to_string(),
                    ports: vec![PortConfig {
                        internal: 80,
                        expose: ExposeType::Http("test.chals.example.com".to_string())
                    }],
                    volume: None
                },
                Pod {
                    name: "bar".to_string(),
                    image_source: ImageSource::Image("nginx".to_string()),
                    replicas: 1,
                    env: ListOrMap::Map(HashMap::from([
                        ("FOO".to_string(), "this".to_string()),
                        ("BAR".to_string(), "that".to_string()),
                    ])),
                    resources: Some(Resource {
                        cpu: 1,
                        memory: "200Mi".to_string()
                    }),
                    architecture: "amd64".to_string(),
                    ports: vec![PortConfig {
                        internal: 80,
                        expose: ExposeType::Http("test2.chals.example.com".to_string())
                    }],
                    volume: None
                }
            ]
        );

        Ok(())
    })
}

#[test]
/// Challenge pod envvar strings error if malformed
fn challenge_pod_bad_env() {
    figment::Jail::expect_with(|jail| {
        jail.create_file("rcds.yaml", VALID_CONFIG)?;
        let dir = jail.create_dir("foo/test")?;
        jail.create_file(
            dir.join("challenge.yaml"),
            r#"
            name: testchal
            author: nobody
            description: just a test challenge
            difficulty: 1

            flag:
                text: test{it-works}

            pods:
                - name: foo
                  image: nginx
                  env:
                    - FOO
                  replicas: 1
                  ports:
                    - internal: 80
                      expose:
                        http: test.chals.example.com
        "#,
        )?;

        let chals = parse_all();
        assert!(chals.is_err());
        let errs = chals.unwrap_err();
        assert_eq!(errs.len(), 1);

        Ok(())
    })
}

#[test]
/// Challenge pod resources override config defaults
fn challenge_pod_resources() {
    figment::Jail::expect_with(|jail| {
        jail.create_file("rcds.yaml", VALID_CONFIG)?;
        let dir = jail.create_dir("foo/test")?;
        jail.create_file(
            dir.join("challenge.yaml"),
            r#"
            name: testchal
            author: nobody
            description: just a test challenge
            difficulty: 1

            flag:
                text: test{it-works}

            pods:
                - name: foo
                  image: nginx
                  replicas: 1
                  resources:
                    cpu: 4
                    memory: 1Gi
                  ports:
                    - internal: 80
                      expose:
                        http: test.chals.example.com
        "#,
        )?;

        let chals = parse_all().unwrap();

        assert_eq!(
            chals[0].pods,
            vec![Pod {
                name: "foo".to_string(),

                image_source: ImageSource::Image("nginx".to_string()),
                replicas: 1,
                env: ListOrMap::Map(HashMap::new()),
                resources: Some(Resource {
                    cpu: 4,
                    memory: "1Gi".to_string()
                }),
                architecture: "amd64".to_string(),
                ports: vec![PortConfig {
                    internal: 80,
                    expose: ExposeType::Http("test.chals.example.com".to_string())
                }],
                volume: None
            }]
        );

        Ok(())
    })
}
