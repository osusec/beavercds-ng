use inquire;
use tera;

struct init_vars
{
    //
}

pub fn run(_interactive: &bool) //-> inquire::error::InquireResult<()>
{
    let options;

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
    else {
        options = noninteractive_init();
    }

    let mut t = tera::Tera::new("src/templates/rcds.yaml.j2").unwrap();
    let ctx = tera::Context::from_serialize(options).unwrap();
    let rendered = t.render("rcds.yaml.j2", &ctx).unwrap();
    println!("{rendered}");
}

fn interactive_init() -> inquire::error::InquireResult<()>
{
    let flag_regex;
    // let registry_domain;
    // let registry_build_user;
    // let registry_build_pass;
    // let registry_cluster_user;
    // let registry_cluster_pass;
    // let defaults_difficulty;
    // let defaults_resources_cpu;
    // let defaults_resources_mem;
    //let points_difficulty
    //let deploy_profile
    // contd

    //println!() about you can press enter to leave blank and all of these
    //  can be set via environment variables also

    flag_regex = inquire::Text::new ("Regex of flags:")
                     .with_help_message("This regex will be used to validate the individual flags of your challenges later.")
                     .prompt()?; // yo is this even a good idea to have the user provide the regex
    println!("{flag_regex}");
    Ok(())
}

fn noninteractive_init() //-> u64
{
    //
}
