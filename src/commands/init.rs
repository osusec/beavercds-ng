use inquire;
use tera;
use std::fmt;
use regex::Regex;

use crate::{access_handlers::frontend, commands::deploy};

struct init_vars
{
    flag_regex: String, // TODO: make all of these `str`s if it compiles
    registry_domain: String,
    registry_build_user: String,
    registry_build_pass: String,
    registry_cluster_user: String,
    registry_cluster_pass: String,
    defaults_difficulty: String, //u64,
    defaults_resources_cpu: String, //u64,
    defaults_resources_memory: String, //(u64, Option(String)),
    points: Vec<points>,
    profiles: Vec<profile>
}

#[derive(Clone)]
struct points {
    difficulty: String, //u64,
    min: String, //u64,
    max: String, //u64
}

impl fmt::Display for points {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}  Points: {}-{})", self.difficulty, self.min, self.max)
    }
}

struct profile {
    profile_name: String,
    frontend_url: String,
    frontend_token: String,
    challenges_domain: String,
    kubecontext: String,
    s3_endpoint: String,
    s3_region: String,
    s3_accesskey: String,
    s3_secretaccesskey: String
}

pub fn run(_interactive: &bool, _blank: &bool) // TODO: is there a way to set options at mutually exclusive?
{
    let options: init_vars;

    if *_interactive {
        options = match interactive_init()
        {
            Ok(t) => t,
            Err(e) =>
            {
                println!("Error in init: {e}");
                return;
            }
        };
    }
    else if *_blank {
        options = blank_init();
    }
    else {
        options = noninteractive_init();
    }

    // TODO -- does not compile because options does not yet implement serialize
    // comment out if wanting to test the rest of the code
    let mut t = tera::Tera::new("src/templates/rcds.yaml.j2").unwrap();
    let ctx = tera::Context::from_serialize(options).unwrap();
    let rendered = t.render("rcds.yaml.j2", &ctx).unwrap();
    println!("{rendered}");
}


fn interactive_init() -> inquire::error::InquireResult<init_vars>
{
    let flag_regex;
    let registry_domain;
    let registry_build_user;
    let registry_build_pass;
    let registry_cluster_user;
    let registry_cluster_pass;
    let defaults_difficulty;
    let defaults_resources_cpu;
    let defaults_resources_memory;
    let mut points_difficulty = Vec::new();
    let mut deploy_profiles = Vec::new();

    println!("For all prompts below, simply press Enter to leave blank.");
    println!("All fields that can be set in rcds.yaml can also be set via environment variables.");

    flag_regex = inquire::Text::new ("Flag regex:")
                    .with_help_message("This regex will be used to validate the individual flags of your challenges later.") // TODO: also provide regex examples for help
                    .prompt()?; // yo is this even a good idea to have the user provide the regex
                    // TODO: with placeholder?
    
    registry_domain = inquire::Text::new ("Container registry:")
                    .with_help_message("This is the domain of your remote container registry, which includes both the endpoint details and your repository name.") //where you will push images to and where your cluster will pull challenge images from.")
                    .prompt()?;

    registry_build_user = inquire::Text::new ("Container registry user (YOURS):")
                    .with_help_message("Your username to the remote container registry, which you will use to push containers to.")
                    .prompt()?;

    // TODO: do we actually want to be in charge of these credentials vs letting the container building utility take care of it?
    registry_build_pass = inquire::Password::new("Container registry password (YOURS):")
                    .with_help_message("Your password to the remote container registry, which you will use to push containers to.") // TODO: could this support username:pat too?
                    .with_display_mode(inquire::PasswordDisplayMode::Masked)
                    .with_custom_confirmation_message("Enter again:")
                    .prompt()?;

    registry_cluster_user = inquire::Text::new ("Container registry user (CLUSTER'S):")
                    .with_help_message("The cluster's username to the remote container registry, which it will use to pull containers from.")
                    .prompt()?;

    // TODO: would the cluster not use a token of some sort?
    registry_cluster_pass = inquire::Password::new("Container registry password (CLUSTER'S):")
                    .with_help_message("The cluster's password to the remote container registry, which it will use to pull containers from.")
                    .prompt()?;

    println!("You can define several challenge difficulty classes below:");
    loop
    {
        // TODO: theres no reason these need to be numbers instead of open strings, e.g. for "easy"
        let difficulty_class_rank = inquire::CustomType::<u64>::new("Difficulty rank:")
                    // default parser calls std::u64::from_str
                    .with_error_message("Please type a valid number.")
                    .with_help_message("The rank of the difficulty class as an unsigned integer, with lower numbers being \"easier.\"")
                    .prompt()?;
        
        // TODO: support static-point challenges
        let difficulty_class_min = inquire::CustomType::<u64>::new("Minimum number of points:")
                    // default parser calls std::u64::from_str
                    .with_error_message("Please type a valid number.")
                    .with_help_message("Challenge points are dynamic: the maximum number of points that challenges within this difficulty class are worth.")
                    .prompt()?;

        let difficulty_class_max = inquire::CustomType::<u64>::new("Maximum number of points:")
                    // default parser calls std::u64::from_str
                    .with_error_message("Please type a valid number.")
                    .with_help_message("Challenge points are dynamic: the minimum number of points that challenges within this difficulty class are worth.")
                    .prompt()?;

        let points_object = points {
            difficulty: difficulty_class_rank.to_string(),
            min: difficulty_class_min.to_string(),
            max: difficulty_class_max.to_string()
        };
        points_difficulty.push(points_object);

        let again = inquire::Confirm::new("Do you want to provide another difficulty class?")
                    .with_default(false)
                    .prompt()?;
        if !again
        {
            break;
        }
    }

    defaults_difficulty = inquire::Select::new("Please choose the default difficulty class:", points_difficulty.clone())
                    .prompt()?;

    // TODO: how much format validation should these two do now vs offloading to validate() later? current inquire replacement calls are temporary and do the zero checking, just grabbing a String
    // defaults_resources_cpu = inquire::CustomType::<u64>::new("Default CPUs per challenge:")
    //                 // default parser calls std::u64::from_str
    //                 .with_error_message("Please type a valid number.")
    //                 .with_help_message("The maximum limit of CPU resources per instance of challenge deployment (\"pod\").")
    //                 .prompt()?;
    defaults_resources_cpu = inquire::Text::new("Default limit of CPUs per challenge")
                        .with_help_message("The maximum limit of CPU resources per instance of challenge deployment (\"pod\").")
                        .prompt()?;
    
    // defaults_resources_memory = inquire::CustomType::<String>::new("")
    //                 .with_parser(&|i| 
    //                 {
    //                     let re = Regex::new(r"^[0-9]+$") // TODO
    //                 })
    defaults_resources_memory = inquire::Text::new("Default limit of memory per challenge")
                        .with_help_message("The maximum limit of memory resources per instance of challenge deployment (\"pod\").")
                        .prompt()?;

    println!("You can define several challenge difficulty classes below.");
    loop {
        let name = inquire::Text::new("Profile name:")
                        .with_help_message("The name of the deployment profile. One profile named \"default\" is recommended. You can add additional profiles.")
                        .prompt()?;
        let frontend_url = inquire::Text::new("Frontend URL:")
                        .with_help_message("The URL of the RNG scoreboard.") // TODO: can definitely say more about why this is significant
                        .prompt()?;

        let frontend_token = inquire::Text::new("Frontend token:")
                        .with_help_message("The token for RNG to authenticate itself into the scoreboard.") // TODO: again, say more
                        .prompt()?;
        
        let challenges_domain = inquire::Text::new("Challenges domain:")
                        .with_help_message("Domain that challenges are hosted under.")
                        .prompt()?;

        let kubecontext = inquire::Text::new("Kube context:")
                        .with_help_message("The name of the context that kubectl looks for to interface with the cluster.")
                        .prompt()?;

        let s3_endpoint = inquire::Text::new("S3 endpoint:")
                        .with_help_message("Challenge artifacts and static files will be hosted on and served from S3. The endpoint of the S3 bucket server.")
                        .prompt()?;

        let s3_region = inquire::Text::new("S3 region:")
                        .with_help_message("The region that the S3 bucket is hosted.")
                        .prompt()?;

        let s3_accesskey = inquire::Text::new("S3 access key:")
                        .with_help_message("The public access key to the S3 bucket.")
                        .prompt()?;

        let s3_secretkey = inquire::Text::new("S3 secret key:")
                        .with_help_message("The secret acess key to the S3 bucket.")
                        .prompt()?;

        let profile_object = profile {
            profile_name: name,
            frontend_url: frontend_url,
            frontend_token: frontend_token,
            challenges_domain: challenges_domain,
            kubecontext: kubecontext,
            s3_endpoint: s3_endpoint,
            s3_region: s3_region,
            s3_accesskey: s3_accesskey,
            s3_secretaccesskey: s3_secretkey
        };
        deploy_profiles.push(profile_object);

        let again = inquire::Confirm::new("Do you want to provide another deployment profile?")
                    .with_default(false)
                    .prompt()?;
        if !again
        {
            break;
        }
    }

    // Put everything into the struct and return it
    let options = init_vars {
        flag_regex: flag_regex,
        registry_domain: registry_domain,
        registry_build_user: registry_build_user,
        registry_build_pass: registry_build_pass,
        registry_cluster_user: registry_cluster_user,
        registry_cluster_pass: registry_cluster_pass,
        defaults_difficulty: defaults_difficulty.difficulty,
        defaults_resources_cpu: defaults_resources_cpu,
        defaults_resources_memory: defaults_resources_memory,
        points: points_difficulty,
        profiles: deploy_profiles
    };

    return Ok(options);
}


fn noninteractive_init() -> init_vars
{
    return init_vars {
        flag_regex: String::from("ctf{.*}"), // TODO: do that wildcard in most common regex flavor since Rust regex supports multiple styles
        registry_domain: String::from("ghcr.io/youraccount"),
        registry_build_user: String::from("admin"),
        registry_build_pass: String::from("notrealcreds"),
        registry_cluster_user: String::from("cluster_user"),
        registry_cluster_pass: String::from("alsofake"),
        defaults_difficulty: String::from("1"),
        defaults_resources_cpu: String::from("1"),
        defaults_resources_memory: String::from("500M"), //(500, Some(String::from("M"))),
        points: vec![
            points {
                difficulty: String::from("1"),
                min: String::from("69"),
                max: String::from("420")
            },
            points {
                difficulty: String::from("2"),
                min: String::from("200"),
                max: String::from("500")
            }
        ],
        profiles: vec![
            profile {
                profile_name: String::from("default"),
                frontend_url: String::from("https://ctf.coolguy.xyz"),
                frontend_token: String::from("secretsecretsecret"),
                challenges_domain: String::from("chals.coolguy.xyz"),
                kubecontext: String::from("ctf-cluster"),
                s3_endpoint: String::from("s3.coolguy.xyz"),
                s3_region: String::from("us-west-2"),
                s3_accesskey: String::from("accesskey"),
                s3_secretaccesskey: String::from("secretkey")
            }
        ]
    };
}


fn blank_init() -> init_vars {
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
        points: vec![
            points {
                difficulty: String::new(),
                min: String::new(),
                max: String::new()
            }
        ],
        profiles: vec![
            profile {
                profile_name: String::new(),
                frontend_url: String::new(),
                frontend_token: String::new(),
                challenges_domain: String::new(),
                kubecontext: String::new(),
                s3_endpoint: String::new(),
                s3_region: String::new(),
                s3_accesskey: String::new(),
                s3_secretaccesskey: String::new(),
            }
        ]
    };
}
