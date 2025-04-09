use simplelog::error;
use std::process::exit;
use std::fs::File;
use std::io::Write;

use crate::{access_handlers::frontend, commands::deploy};
use crate::init::{self as init, templatize_init};


pub fn run(_interactive: &bool, _blank: &bool)
{
    let options: init::init_vars;

    if *_interactive
    {
        options = match init::interactive_init()
        {
            Ok(t) => t,
            Err(e) =>
            {
                error!("Error in init: {e}");
                exit(1);
            }
        };
    }
    else if *_blank
    {
        options = init::blank_init();
    }
    else {
        options = init::example_init();
    }

    // TODO write to disk
    let configuration = templatize_init(options);
    let mut f = match File::create("rcds.yaml")
    {
        Ok(t) => t,
        Err(e) =>
        {
            error!("Error in init: {e}");
            exit(1);
        }
    };
    match f.write_all(configuration.as_bytes())
    {
        Ok(_) => (),
        Err(e) => 
        {
            error!("Error in init: {e}");
            exit(1);
        }
    }

}
