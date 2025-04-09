use inquire;
use minijinja;
use regex::Regex;
use serde;
use std::fmt;

pub mod templates;

#[derive(serde::Serialize)]
pub struct init_vars {
    pub flag_regex: String,
    pub registry_domain: String,
    pub registry_build_user: String,
    pub registry_build_pass: String,
    pub registry_cluster_user: String,
    pub registry_cluster_pass: String,
    pub defaults_difficulty: String,       //u64,
    pub defaults_resources_cpu: String,    //u64,
    pub defaults_resources_memory: String, //(u64, Option(String)),
    pub points: Vec<points>,
    pub profiles: Vec<profile>,
}

#[derive(Clone, serde::Serialize)]
pub struct points {
    pub difficulty: String, //u64,
    pub min: String,        //u64,
    pub max: String,        //u64
}

impl fmt::Display for points {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "({}  Points: {}-{})",
            self.difficulty, self.min, self.max
        )
    }
}

#[derive(serde::Serialize)]
pub struct profile {
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
    // TODO external dns garbage
    pub dns_provider: String,
    // dns_provider_values: HashMap<String, String>
    // dns_txtOwnerId: Option<
}

pub fn interactive_init() -> inquire::error::InquireResult<init_vars> {
    println!("For all prompts below, simply press Enter to leave blank.");
    println!("All fields that can be set in rcds.yaml can also be set via environment variables.");

    let points_ranks_reference: Vec<points>;

    let options = init_vars {
        flag_regex: {
            //TODO:
            // - also provide regex examples in help
            // - is this even a good idea to have the user provide the regex
            // - with placeholder?
            inquire::Text::new("Flag regex:")
            .with_help_message("This regex will be used to validate the individual flags of your challenges later.")
            .prompt()?
        },

        registry_domain: {
            inquire::Text::new ("Container registry:")
            .with_help_message("This is the domain of your remote container registry, which includes both the endpoint details and your repository name.") //where you will push images to and where your cluster will pull challenge images from.") // TODO
            .prompt()?
        },

        registry_build_user: {
            inquire::Text::new ("Container registry user (YOURS):")
            .with_help_message("Your username to the remote container registry, which you will use to push containers to.")
            .prompt()?
        },

        // TODO: do we actually want to be in charge of these credentials vs letting the container building utility take care of it?
        registry_build_pass: {
            inquire::Password::new("Container registry password (YOURS):")
            .with_help_message("Your password to the remote container registry, which you will use to push containers to.") // TODO: could this support username:pat too?
            .with_display_mode(inquire::PasswordDisplayMode::Masked)
            .with_custom_confirmation_message("Enter again:")
            .prompt()?
        },

        registry_cluster_user: {
            inquire::Text::new ("Container registry user (CLUSTER'S):")
            .with_help_message("The cluster's username to the remote container registry, which it will use to pull containers from.")
            .prompt()?
        },

        // TODO: would the cluster not use a token of some sort?
        registry_cluster_pass: {
            inquire::Password::new("Container registry password (CLUSTER'S):")
            .with_help_message("The cluster's password to the remote container registry, which it will use to pull containers from.")
            .with_display_mode(inquire::PasswordDisplayMode::Masked)
            .with_custom_confirmation_message("Enter again:")
            .prompt()?
        },

        points: {
            println!("You can define several challenge difficulty classes below.");
            let mut again = inquire::Confirm::new("Do you want to provide a difficulty class?")
                .with_default(false)
                .prompt()?;
            let mut points_ranks: Vec<points> = Vec::new();
            while again {
                let points_obj = points {
                    // TODO: theres no reason these need to be numbers instead of open strings, e.g. for "easy"
                    difficulty: {
                        inquire::CustomType::<u64>::new("Difficulty rank:")
                        // default parser calls std::u64::from_str
                        .with_error_message("Please type a valid number.")
                        .with_help_message("The rank of the difficulty class as an unsigned integer, with lower numbers being \"easier.\"")
                        .prompt()?
                        .to_string()
                    },
                    // TODO: support static-point challenges
                    min: {
                        inquire::CustomType::<u64>::new("Minimum number of points:")
                        // default parser calls std::u64::from_str
                        .with_error_message("Please type a valid number.")
                        .with_help_message("Challenge points are dynamic: the minimum number of points that challenges within this difficulty class are worth.")
                        .prompt()?
                        .to_string()
                    },
                    max: {
                        inquire::CustomType::<u64>::new("Maximum number of points:")
                        // default parser calls std::u64::from_str
                        .with_error_message("Please type a valid number.")
                        .with_help_message("Challenge points are dynamic: the maximum number of points that challenges within this difficulty class are worth.")
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

        // TODO: how much format validation should these two do now vs offloading to validate() later? current inquire replacement calls are temporary and do the zero checking, just grabbing a String
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
            let resources = inquire::Text::new("Default limit of CPUs per challenge:")
            .with_help_message("The maximum limit of CPU resources per instance of challenge deployment (\"pod\").")
            .with_placeholder("1")
            .prompt()?;

            if resources.is_empty() {
                String::from("1")
            } else {
                resources
            }
        },

        defaults_resources_memory: {
            let resources = inquire::Text::new("Default limit of memory per challenge:")
            .with_help_message("The maximum limit of memory resources per instance of challenge deployment (\"pod\").")
            .with_placeholder("500M")
            .prompt()?;

            if resources.is_empty() {
                String::from("500M")
            } else {
                resources
            }
        },

        profiles: {
            println!("You can define several environment profiles below.");

            let mut again = inquire::Confirm::new("Do you want to provide a profile?")
                .with_default(false)
                .prompt()?;
            let mut profiles: Vec<profile> = Vec::new();
            while again {
                let prof = profile {
                    profile_name: {
                        inquire::Text::new("Profile name:")
                        .with_help_message("The name of the deployment profile. One profile named \"default\" is recommended. You can add additional profiles.")
                        .with_placeholder("default")
                        .prompt()?
                    },
                    frontend_url: {
                        inquire::Text::new("Frontend URL:")
                            .with_help_message("The URL of the RNG scoreboard.") // TODO: can definitely say more about why this is significant
                            .prompt()?
                    },
                    frontend_token: {
                        inquire::Text::new("Frontend token:")
                            .with_help_message(
                                "The token for RNG to authenticate itself into the scoreboard.",
                            ) // TODO: again, say more
                            .prompt()?
                    },
                    challenges_domain: {
                        inquire::Text::new("Challenges domain:")
                            .with_help_message("Domain that challenges are hosted under.")
                            .prompt()?
                    },
                    kubecontext: {
                        inquire::Text::new("Kube context:")
                        .with_help_message("The name of the context that kubectl looks for to interface with the cluster.")
                        .prompt()?
                    },
                    s3_bucket_name: {
                        inquire::Text::new("S3 bucket name:")
                        .with_help_message("Challenge artifacts and static files will be hosted on and served from S3. The name of the S3 bucket.")
                        .prompt()?
                    },
                    s3_endpoint: {
                        inquire::Text::new("S3 endpoint:")
                            .with_help_message("The endpoint of the S3 bucket server.")
                            .prompt()?
                    },
                    s3_region: {
                        inquire::Text::new("S3 region:")
                            .with_help_message("The region that the S3 bucket is hosted.")
                            .prompt()?
                    },
                    s3_accesskey: {
                        inquire::Text::new("S3 access key:")
                            .with_help_message("The public access key to the S3 bucket.")
                            .prompt()?
                    },
                    s3_secretaccesskey: {
                        inquire::Text::new("S3 secret key:")
                            .with_help_message("The secret acess key to the S3 bucket.")
                            .prompt()?
                    },
                    dns_provider: {
                        // TODO : literally all of the external DNS settings
                        inquire::Text::new("DNS provider:")
                            .with_help_message("The name of the cloud DNS provider being used.")
                            .with_placeholder("route53")
                            .prompt()?
                    },
                };
                profiles.push(prof);

                again = inquire::Confirm::new("Do you want to provide another profile?")
                    .with_default(false)
                    .prompt()?;
            }
            profiles
        },
    };
    return Ok(options);
}

pub fn blank_init() -> init_vars {
    return init_vars {
        flag_regex: String::new(),
        registry_domain: String::new(),
        registry_build_user: String::new(),
        registry_build_pass: String::new(),
        registry_cluster_user: String::new(),
        registry_cluster_pass: String::new(),
        defaults_difficulty: String::new(),
        defaults_resources_cpu: String::new(),
        defaults_resources_memory: String::new(),
        points: vec![points {
            difficulty: String::new(),
            min: String::new(),
            max: String::new(),
        }],
        profiles: vec![profile {
            profile_name: String::from("default"),
            frontend_url: String::new(),
            frontend_token: String::new(),
            challenges_domain: String::new(),
            kubecontext: String::new(),
            s3_bucket_name: String::new(),
            s3_endpoint: String::new(),
            s3_region: String::new(),
            s3_accesskey: String::new(),
            s3_secretaccesskey: String::new(),
            dns_provider: String::from("aws"),
        }],
    };
}

pub fn example_init() -> init_vars {
    return init_vars {
        flag_regex: String::from("ctf{.*}"), // TODO: do that wildcard in the most common regex flavor since Rust regex supports multiple styles
        registry_domain: String::from("ghcr.io/youraccount"),
        registry_build_user: String::from("admin"),
        registry_build_pass: String::from("notrealcreds"),
        registry_cluster_user: String::from("cluster_user"),
        registry_cluster_pass: String::from("alsofake"),
        defaults_difficulty: String::from("1"),
        defaults_resources_cpu: String::from("1"),
        defaults_resources_memory: String::from("500M"),
        points: vec![
            points {
                difficulty: String::from("1"),
                min: String::from("1"),
                max: String::from("1337"),
            },
            points {
                difficulty: String::from("2"),
                min: String::from("200"),
                max: String::from("500"),
            },
        ],
        profiles: vec![profile {
            profile_name: String::from("default"),
            frontend_url: String::from("https://ctf.coolguy.xyz"),
            frontend_token: String::from("secretsecretsecret"),
            challenges_domain: String::from("chals.coolguy.xyz"),
            kubecontext: String::from("ctf-cluster"),
            s3_bucket_name: String::from("ctf-bucket"),
            s3_endpoint: String::from("s3.coolguy.xyz"),
            s3_region: String::from("us-west-2"),
            s3_accesskey: String::from("accesskey"),
            s3_secretaccesskey: String::from("secretkey"),
            dns_provider: String::from("aws"),
        }],
    };
}

pub fn templatize_init(options: init_vars) -> String {
    let filled_template = minijinja::render!(templates::RCDS, options);
    return filled_template;
}
