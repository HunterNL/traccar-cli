use std::{io::stdin, process::exit};

use traccar_lib::TracarrError;

use crate::config::{ConfigBase, ConfigFile};

fn await_input() -> String {
    let mut out = String::new();
    stdin().read_line(&mut out).expect("error in readline");
    out.trim().to_owned()
}

fn await_secret_input() -> String {
    rpassword::read_password()
        .expect("error in rpassword")
        .trim()
        .to_owned()
}

pub fn run(config_base: &ConfigBase) -> Option<TracarrError> {
    let current_config = config_base.read_config_file();

    let current_host = current_config.as_ref().and_then(|f| f.host.clone());
    let current_token = current_config.as_ref().and_then(|c| c.token.clone());
    let device_config = current_config.and_then(|f| f.devices);

    // Ask for hostname
    let host = {
        match &current_host {
            Some(host) => {
                println!("Enter hostname or leave empty to keep {host}:");
            }
            None => {
                println!("Enter hostname:");
            }
        }

        let input = await_input();
        if input.is_empty() {
            current_host
        } else {
            Some(input)
        }
    };

    if host.is_none() {
        println!("no hostname given");
        exit(1);
    }

    // Ask for token
    let token = {
        match &current_token {
            Some(_) => {
                println!("Enter token or leave empty to keep as is:");
            }
            None => {
                println!("Enter token:");
            }
        }

        let input = await_secret_input();
        if input.is_empty() {
            current_token
        } else {
            Some(input)
        }
    };

    let new_config: ConfigFile = ConfigFile {
        host,
        token,
        devices: device_config,
    };

    config_base.write_config_file(&new_config);

    println!("Info saved");

    None
}
