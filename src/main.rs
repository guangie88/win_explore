#[macro_use]
extern crate error_chain;
extern crate libc;

#[macro_use]
extern crate log;
extern crate log4rs;
extern crate serde_json;
extern crate structopt;

#[macro_use]
extern crate structopt_derive;

use std::ffi::CString;
use std::io::{self, Read, Write};
use std::path::Path;
use std::process::{self, Command};
use std::net::TcpListener;
use structopt::StructOpt;

mod errors {
    error_chain! {
        errors {
        }
    }
}

use errors::*;

#[derive(StructOpt, Debug)]
#[structopt(name = "Windows Explore Agent", about = "Windows Explore TCP Listening Agent.")]
struct MainConfig {
    #[structopt(short = "a", long = "address", help = "Interface address to host", default_value = "0.0.0.0")]
    address: String,

    #[structopt(short = "p", long = "port", help = "Port to host", default_value = "22222")]
    port: u16,

    #[structopt(short = "l", long = "log-config-path", help = "Log config file path")]
    log_config_path: String,
}

fn run() -> Result<()> {
    // initialization
    let config = MainConfig::from_args();

    let _ = log4rs::init_file(&config.log_config_path, Default::default())
       .chain_err(|| format!("Unable to initialize log4rs logger with the given config file at '{}'", config.log_config_path))?;

    info!("Config: {:?}", config);

    // test
    {
        let content = "cmd /C D:\\batch_exec".to_string();
        let content_ref: &[u8] = content.as_ref();
        let content_vec: Vec<u8> = content_ref.into();
        let c_content = CString::new(content_vec);

        match c_content {
            Ok(c_content) => {
                let exit_code = unsafe { libc::system(c_content.as_ptr()) };
            },

            Err(e) => error!("Unable to convert from Rust string to C string: {}", e), 
        }

        /*let child = Command::new("cmd")
            .args(&["/C", &content])
            .spawn();

        match child {
            Ok(mut child) => {
                match child.wait() {
                    Ok(exit_status) => info!("Directory path '{}' explored with exit status: {:?}", content, exit_status),
                    Err(e) => error!("Error waiting for exploring result: {}", e),
                }
            },

            Err(e) => error!("Unable to spawn child with directory path '{}': {}", content, e),
        }*/
    }

    // body content
    let listener = TcpListener::bind((config.address.as_str(), config.port))
        .chain_err(|| format!("Unable to bind TCP listener at '{}:{}'", config.address, config.port))?;

    for stream in listener.incoming() {
        let content = stream.map(|mut stream| {
            let mut content = String::new();
            let _ = stream.read_to_string(&mut content);
            content
        });

        match content {
            Ok(content) => {
                let tmp_dir_path = Path::new(&content);

                match tmp_dir_path.is_dir() {
                    true => {
                        let child = Command::new("cmd")
                            .args(&["/C", &content])
                            .spawn();

                        match child {
                            Ok(mut child) => {
                                match child.wait() {
                                    Ok(exit_status) => info!("Directory path '{}' explored with exit status: {:?}", content, exit_status),
                                    Err(e) => error!("Error waiting for exploring result: {}", e),
                                }
                            },

                            Err(e) => error!("Unable to spawn child with directory path '{}': {}", content, e),
                        }
                    },

                    false => error!("Stream content not a directory path: {}", content),
                }
            },

            Err(e) => error!("Stream error: {}", e),
        }
    }

    Ok(())
}

fn main() {
    match run() {
        Ok(_) => {
            println!("Program completed!");
            process::exit(0)
        },

        Err(ref e) => {
            let stderr = &mut io::stderr();

            writeln!(stderr, "Error: {}", e)
                .expect("Unable to write error into stderr!");

            for e in e.iter().skip(1) {
                writeln!(stderr, "- Caused by: {}", e)
                    .expect("Unable to write error causes into stderr!");
            }

            process::exit(1);
        },
    }
}
