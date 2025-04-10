use inquire;
use minijinja;
use regex::Regex;
use serde;
use std::fmt;

pub mod templates;
pub mod example_values;

#[derive(serde::Serialize)]
pub struct InitVars {
    pub flag_regex: String,
    pub registry_domain: String,
    pub registry_build_user: String,
    pub registry_build_pass: String,
    pub registry_cluster_user: String,
    pub registry_cluster_pass: String,
    pub defaults_difficulty: String,
    pub defaults_resources_cpu: String,
    pub defaults_resources_memory: String,
    pub points: Vec<Points>,
    pub profiles: Vec<Profile>,
}

#[derive(Clone, serde::Serialize)]
pub struct Points {
    pub difficulty: String,
    pub min: String,
    pub max: String,
}

impl fmt::Display for Points {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "({}  Points: {}-{})",
            self.difficulty, self.min, self.max
        )
    }
}

#[derive(serde::Serialize)]
pub struct Profile {
    pub profile_name: String,
    pub frontend_url: String,
    pub frontend_token: String,
    pub challenges_domain: String,
    pub kubecontext: String,
    pub s3_bucket_name: String,
    pub s3_endpoint: String,
    pub s3_region: String,
    pub s3_accesskey: String,
    pub s3_secretaccesskey: String,
}

pub fn interactive_init() -> inquire::error::InquireResult<InitVars> {
    println!("For all prompts below, simply press Enter to leave blank.");
    println!("All fields that can be set in rcds.yaml can also be set via environment variables.");

    let points_ranks_reference: Vec<Points>;

    let options = InitVars {
        flag_regex: {
            //TODO: what flavor of regex is being validated and accepted
            inquire::Text::new("Flag regex:")
            .with_help_message("This regex will be used to validate the individual flags of your challenges later.")
            .with_placeholder(example_values::FLAG_REGEX)
            .prompt()?
        },

        registry_domain: {
            inquire::Text::new ("Container registry:")
            .with_help_message("Hosted challenges will be hosted in a container registry.The connection endpoint and the repository name.") 
            .with_placeholder(example_values::REGISTRY_DOMAIN)
            .prompt()?
        },

        registry_build_user: {
            inquire::Text::new ("Container registry 'build' user:")
            .with_help_message("The username that will be used to push built containers.")
            .with_placeholder(example_values::REGISTRY_BUILD_USER)
            .prompt()?
        },

        // TODO: do we actually want to be in charge of these credentials vs expecting the local building utility already be logged in?
        registry_build_pass: {
            inquire::Password::new("Container registry 'build' password:")
            .with_help_message("The password to the 'build' user account") // TODO: could this support username:pat too?
            .with_display_mode(inquire::PasswordDisplayMode::Masked)
            .with_custom_confirmation_message("Enter again:")
            .prompt()?
        },

        registry_cluster_user: {
            inquire::Text::new ("Container registry 'cluster' user:")
            .with_help_message("The username that the cluster will use to pull locally-built containers.")
            .with_placeholder(example_values::REGISTRY_CLUSTER_USER)
            .prompt()?
        },

        registry_cluster_pass: {
            inquire::Password::new("Container registry 'cluster' password:")
            .with_help_message("The password to the 'cluster' user account")
            .with_display_mode(inquire::PasswordDisplayMode::Masked)
            .with_custom_confirmation_message("Enter again:")
            .prompt()?
        },

        points: {
            println!("You can define several challenge difficulty classes below.");
            let mut again = inquire::Confirm::new("Do you want to provide a difficulty class?")
                .with_default(false)
                .prompt()?;
            println!("Challenge points are dynamic. For a static challenge, simply set minimum and maximum points to the same value.");
            let mut points_ranks: Vec<Points> = Vec::new();
            while again {
                let points_obj = Points {
                    difficulty: {
                        inquire::Text::new("Difficulty class:")
                        .with_validator(inquire::required!("Please provide a name."))
                        .with_help_message("The name of the difficulty class.")
                        .with_placeholder(example_values::POINTS_DIFFICULTY)
                        .prompt()?
                    },
                    min: {
                        inquire::CustomType::<u64>::new("Minimum points:")
                        .with_error_message("Please type a valid number.") // default parser calls std::u64::from_str
                        .with_help_message("The minimum number of points that challenges within this difficulty class are worth.")
                        .with_placeholder(example_values::POINTS_MIN)
                        .prompt()?
                        .to_string()
                    },
                    max: {
                        inquire::CustomType::<u64>::new("Maximum points:")
                        .with_error_message("Please type a valid number.") // default parser calls std::u64::from_str
                        .with_help_message("The maximum number of points that challenges within this difficulty class are worth.")
                        .with_placeholder(example_values::POINTS_MAX)
                        .prompt()?
                        .to_string()
                    },
                };
                points_ranks.push(points_obj);

                again = inquire::Confirm::new("Do you want to provide another difficulty class?")
                    .with_default(false)
                    .prompt()?;
            }
            points_ranks_reference = points_ranks.clone();
            points_ranks
        },

        defaults_difficulty: {
            if points_ranks_reference.is_empty() {
                String::new()
            } else {
                inquire::Select::new(
                    "Please choose the default difficulty class:",
                    points_ranks_reference,
                )
                .prompt()?
                .difficulty
            }
        },

        defaults_resources_cpu: {
            let resources = inquire::Text::new("Default CPU limit:")
            .with_help_message("The default limit of CPU resources per challenge pod.\nhttps://kubernetes.io/docs/concepts/configuration/manage-resources-containers/#resource-units-in-kubernetes")
            .with_placeholder(example_values::DEFAULTS_RESOURCES_CPU)
            .prompt()?;

            if resources.is_empty() {
                String::from(example_values::DEFAULTS_RESOURCES_CPU)
            } else {
                resources
            }
        },

        defaults_resources_memory: {
            let resources = inquire::Text::new("Default memory limit:")
            .with_help_message("The default limit of CPU resources per challenge pod.\nhttps://kubernetes.io/docs/concepts/configuration/manage-resources-containers/#resource-units-in-kubernetes")
            .with_placeholder(example_values::DEFAULTS_RESOURCES_MEMORY)
            .prompt()?;

            if resources.is_empty() {
                String::from(example_values::DEFAULTS_RESOURCES_MEMORY)
            } else {
                resources
            }
        },

        profiles: {
            println!("You can define several environment profiles below.");

            let mut again = inquire::Confirm::new("Do you want to provide a Profile?")
                .with_default(false)
                .prompt()?;
            let mut profiles: Vec<Profile> = Vec::new();
            while again {
                let prof = Profile {
                    profile_name: {
                        inquire::Text::new("Profile name:")
                        .with_help_message("The name of the deployment Profile. One Profile named \"default\" is recommended. You can add additional profiles.")
                        .with_placeholder(example_values::PROFILES_PROFILE_NAME)
                        .prompt()?
                    },
                    frontend_url: {
                        inquire::Text::new("Frontend URL:")
                            .with_help_message("The URL of the RNG scoreboard.")
                            .with_placeholder(example_values::PROFILES_FRONTEND_URL)
                            .prompt()?
                    },
                    frontend_token: {
                        inquire::Text::new("Frontend token:")
                            .with_help_message(
                                "The token to authenticate into the RNG scoreboard.",
                            )
                            .with_placeholder(example_values::PROFILES_FRONTEND_TOKEN)
                            .prompt()?
                    },
                    challenges_domain: {
                        inquire::Text::new("Challenges domain:")
                            .with_help_message("Domain that challenges are hosted under.")
                            .with_placeholder(example_values::PROFILES_CHALLENGES_DOMAIN)
                            .prompt()?
                    },
                    kubecontext: {
                        inquire::Text::new("Kubecontext name:")
                        .with_help_message("The name of the context that kubectl uses to connect to the cluster.")
                        .with_placeholder(example_values::PROFILES_KUBECONTEXT)
                        .prompt()?
                    },
                    s3_bucket_name: {
                        inquire::Text::new("S3 bucket name:")
                        .with_help_message("Challenge artifacts and static files will be hosted on S3. The name of the S3 bucket.")
                        .with_placeholder(example_values::PROFILES_S3_BUCKET_NAME)
                        .prompt()?
                    },
                    s3_endpoint: {
                        inquire::Text::new("S3 endpoint:")
                            .with_help_message("The endpoint of the S3 bucket server.")
                            .with_placeholder(example_values::PROFILES_S3_ENDPOINT)
                            .prompt()?
                    },
                    s3_region: {
                        inquire::Text::new("S3 region:")
                            .with_help_message("The region where the S3 bucket is hosted.")
                            .with_placeholder(example_values::PROFILES_S3_REGION)
                            .prompt()?
                    },
                    s3_accesskey: {
                        inquire::Text::new("S3 access key:")
                            .with_help_message("The public access key to the S3 bucket.")
                            .with_placeholder(example_values::PROFILES_S3_ACCESSKEY)
                            .prompt()?
                    },
                    s3_secretaccesskey: {
                        inquire::Text::new("S3 secret key:")
                            .with_help_message("The secret acess key to the S3 bucket.")
                            .with_placeholder(example_values::PROFILES_S3_SECRETACCESSKEY)
                            .prompt()?
                    },
                };
                profiles.push(prof);

                again = inquire::Confirm::new("Do you want to provide another Profile?")
                    .with_default(false)
                    .prompt()?;
            }
            profiles
        },
    };
    return Ok(options);
}

pub fn blank_init() -> InitVars {
    return InitVars {
        flag_regex: String::new(),
        registry_domain: String::new(),
        registry_build_user: String::new(),
        registry_build_pass: String::new(),
        registry_cluster_user: String::new(),
        registry_cluster_pass: String::new(),
        defaults_difficulty: String::new(),
        defaults_resources_cpu: String::new(),
        defaults_resources_memory: String::new(),
        points: vec![Points {
            difficulty: String::new(),
            min: String::new(),
            max: String::new(),
        }],
        profiles: vec![Profile {
            profile_name: String::from(example_values::PROFILES_PROFILE_NAME),
            frontend_url: String::new(),
            frontend_token: String::new(),
            challenges_domain: String::new(),
            kubecontext: String::new(),
            s3_bucket_name: String::new(),
            s3_endpoint: String::new(),
            s3_region: String::new(),
            s3_accesskey: String::new(),
            s3_secretaccesskey: String::new(),
        }],
    };
}

pub fn example_init() -> InitVars {
    return InitVars {
        flag_regex: String::from(example_values::FLAG_REGEX),
        registry_domain: String::from(example_values::REGISTRY_DOMAIN),
        registry_build_user: String::from(example_values::REGISTRY_BUILD_USER),
        registry_build_pass: String::from(example_values::REGISTRY_BUILD_PASS),
        registry_cluster_user: String::from(example_values::REGISTRY_CLUSTER_USER),
        registry_cluster_pass: String::from(example_values::REGISTRY_CLUSTER_USER),
        defaults_difficulty: String::from(example_values::DEFAULTS_DIFFICULTY),
        defaults_resources_cpu: String::from(example_values::DEFAULTS_RESOURCES_CPU),
        defaults_resources_memory: String::from(example_values::DEFAULTS_RESOURCES_MEMORY),
        points: vec![
            Points {
                difficulty: String::from(example_values::POINTS_DIFFICULTY),
                min: String::from(example_values::POINTS_MIN),
                max: String::from(example_values::POINTS_MAX),
            },
            Points {
                difficulty: String::from("2"),
                min: String::from("1"),
                max: String::from("1337"),
            },
        ],
        profiles: vec![Profile {
            profile_name: String::from(example_values::PROFILES_PROFILE_NAME),
            frontend_url: String::from(example_values::PROFILES_FRONTEND_URL),
            frontend_token: String::from(example_values::PROFILES_FRONTEND_TOKEN),
            challenges_domain: String::from(example_values::PROFILES_CHALLENGES_DOMAIN),
            kubecontext: String::from(example_values::PROFILES_KUBECONTEXT),
            s3_bucket_name: String::from(example_values::PROFILES_S3_BUCKET_NAME),
            s3_endpoint: String::from(example_values::PROFILES_S3_ENDPOINT),
            s3_region: String::from(example_values::PROFILES_S3_REGION),
            s3_accesskey: String::from(example_values::PROFILES_S3_ACCESSKEY),
            s3_secretaccesskey: String::from(example_values::PROFILES_S3_SECRETACCESSKEY),
        }],
    };
}

pub fn templatize_init(options: InitVars) -> String {
    let filled_template = minijinja::render!(templates::RCDS, options);
    return filled_template;
}
