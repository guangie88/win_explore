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
extern crate winapi;

use std::io::{self, Read, Write};
use std::process;
use std::net::TcpStream;
use structopt::StructOpt;

mod errors {
    error_chain! {
        errors {
        }
    }
}

use errors::*;

#[derive(StructOpt, Debug)]
#[structopt(name = "Windows Explore Client", about = "Windows Explore TCP Client Agent.")]
struct MainConfig {
    #[structopt(short = "a", long = "address", help = "Interface address to host", default_value = "127.0.0.1")]
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

    // body content
    let addr_port_array = [(config.address.as_str(), config.port)];

    let tcp_cycle = addr_port_array.into_iter().cycle()
        .map(|&(addr, port)| -> Result<String> {
            let mut stream = TcpStream::connect((addr, port))
                .chain_err(|| format!("Unable to connect TCP stream at '{}:{}'", addr, port))?;

            // send directory path to server
            print!("Enter directory to explore: ");
            let mut dir_path = String::new();

            io::stdin().read_to_string(&mut dir_path)
                .chain_err(|| "Unable to read string from stdin")?;
            
            let dir_path = dir_path;

            stream.write_fmt(format_args!("{}", dir_path))
                .chain_err(|| "Unable to write buffer string into stream")?;
            
            // receive status from server
            let mut status = String::new();

            stream.read_to_string(&mut status)
                .chain_err(|| "Unable to read status from server stream")?;

            let status = status;

            if status != "OK" {
                bail!(format!("{}", status));
            }

            Ok(dir_path)
        });

    for status_res in tcp_cycle {
        match status_res {
            Ok(dir_path) => {
                info!("'{}' okay to explore!", dir_path);
            },

            Err(e) => {
                error!("Stream error: {}", e);
            },
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
