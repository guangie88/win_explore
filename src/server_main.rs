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

use std::ffi::CString;
use std::io::{self, Read, Write};
use std::path::Path;
use std::ptr;
use std::process;
use std::net::TcpListener;
use structopt::StructOpt;
use winapi::basetsd::INT_PTR;
use winapi::minwindef::{HINSTANCE, INT};
use winapi::windef::HWND;
use winapi::winnt::LPCSTR;
use winapi::winuser::SW_SHOW;

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

#[link(name = "shell32")]
extern {
    // using the non-unicode variant
    fn ShellExecuteA(
        hwnd: HWND,
        lpOperation: LPCSTR,
        lpFile: LPCSTR,
        lpParameters: LPCSTR,
        lpDirectory: LPCSTR,
        nShowCmd: INT) -> HINSTANCE;
}

fn run() -> Result<()> {
    // initialization
    let config = MainConfig::from_args();

    let _ = log4rs::init_file(&config.log_config_path, Default::default())
       .chain_err(|| format!("Unable to initialize log4rs logger with the given config file at '{}'", config.log_config_path))?;

    info!("Config: {:?}", config);

    // body content
    let listener = TcpListener::bind((config.address.as_str(), config.port))
        .chain_err(|| format!("Unable to bind TCP listener at '{}:{}'", config.address, config.port))?;

    let tcp_cycle = listener.incoming()
        .map(|stream| -> Result<String> {
            let mut stream = stream.chain_err(|| "Unable to get a valid stream")?;

            let mut dir_path = String::new();

            stream.read_to_string(&mut dir_path)
                .chain_err(|| "Unable to read string from TCP stream")?;

            let dir_path = dir_path;

            // scope here because need to return directory path at the end
            {
                // check for valid directory first
                let dir_path_fs = Path::new(&dir_path);

                if dir_path_fs.is_dir() {
                    bail!(format!("Given path is not a directory: {:?}", dir_path));
                }

                let dir_path_ref: &[u8] = dir_path.as_ref();
                let dir_path_vec: Vec<u8> = dir_path_ref.into();
                let c_dir_path = CString::new(dir_path_vec).unwrap();

                // the return value must be interpreted as INT_PTR
                // success if the return code is > 32
                // otherwise failure code needs to be referred on Windows API
                let shell_ret_code = unsafe {
                    ShellExecuteA(
                        ptr::null_mut(),
                        ptr::null_mut(),
                        c_dir_path.as_ptr() as *const i8,
                        ptr::null_mut(),
                        ptr::null_mut(),
                        SW_SHOW) as INT_PTR
                };

                const CRITERION: INT_PTR = 32 as INT_PTR;

                let status = if shell_ret_code > CRITERION {
                    "OK".to_string()
                } else {
                    format!("Error shell executing the given directory '{}', error code: {:?}", dir_path, shell_ret_code)
                };

                stream.write_fmt(format_args!("{}", status))
                    .chain_err(|| "Unable to write buffer string into stream")?;
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
