use std::{io::stdin, process::exit};

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

pub fn run(config_base: ConfigBase) {
    let config_file = config_base.read_config_file();
    let host = {
        match &config_file.host {
            Some(host) => {
                println!("Enter hostname or leave empty to keep {host}:");
            }
            None => {
                println!("Enter hostname:");
            }
        }

        let input = await_input();
        if input.is_empty() {
            config_file.host.clone()
        } else {
            Some(input)
        }
    };

    if host.is_none() {
        println!("no hostname given");
        exit(1);
    }

    let token = {
        match &config_file.token {
            Some(_) => {
                println!("Enter token or leave empty to keep as is:");
            }
            None => {
                println!("Enter token:");
            }
        }

        let input = await_secret_input();
        if input.is_empty() {
            config_file.token.clone()
        } else {
            Some(input)
        }
    };

    let config_file: ConfigFile = ConfigFile {
        host,
        token,
        devices: config_file.devices.clone(),
    };

    config_base.write_config_file(&config_file);

    println!("Info saved");
}
