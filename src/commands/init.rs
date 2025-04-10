use simplelog::error;
use std::fs::File;
use std::io::Write;
use std::process::exit;

use crate::init::{self as init, templatize_init};
use crate::{access_handlers::frontend, commands::deploy};

pub fn run(_interactive: &bool, _blank: &bool) {
    let options: init::InitVars;

    if *_interactive {
        options = match init::interactive_init() {
            Ok(t) => t,
            Err(e) => {
                error!("Error in init: {e}");
                exit(1);
            }
        };
    } else if *_blank {
        options = init::blank_init();
    } else {
        options = init::example_init();
    }

    let configuration = templatize_init(options);
    let mut f = match File::create("rcds.yaml") {
        Ok(t) => t,
        Err(e) => {
            error!("Error in init: {e}");
            exit(1);
        }
    };
    match f.write_all(configuration.as_bytes()) {
        Ok(_) => (),
        Err(e) => {
            error!("Error in init: {e}");
            exit(1);
        }
    }

    // Note about external-dns
    println!("Note: external-dns configuration settings will need to be provided in rcds.yaml after file creation, under the `profiles.name.dns` key.");
    println!("Reference: https://github.com/bitnami/charts/tree/main/bitnami/external-dns");
}
