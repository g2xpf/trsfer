use std::fs;
use std::io::{self, ErrorKind};
use std::net::TcpListener;
use std::path::Path;
use std::sync::Arc;
use std::thread::Builder;

use clap::ArgMatches;

use crate::stream_handler;

use super::DEFAULT_PORT;

const DEFAULT_OUTPUT_PATH: &str = ".";

struct TrsferServerConfig<'a> {
    output_path: &'a Path,
    port: u16,
    allow_port_fallback: bool,
}

pub fn run(matches: &ArgMatches<'_>) {
    let port: u16;
    let allow_port_fallback;

    if matches.occurrences_of("port") > 0 {
        let port_arg = matches.value_of("port").unwrap();
        if let Ok(port_arg) = port_arg.parse() {
            port = port_arg;
            allow_port_fallback = false;
        } else {
            exit!(1, port_arg);
        }
    } else {
        port = DEFAULT_PORT;
        allow_port_fallback = true;
    }

    let output_path = match matches.value_of("output") {
        Some(output_path_arg) => {
            let output_path = Path::new(output_path_arg);
            if output_path.is_dir() {
                output_path
            } else {
                exit!(7, output_path_arg);
            }
        }
        None => Path::new(DEFAULT_OUTPUT_PATH),
    };

    // prepare the output directory if not exists
    if !output_path.exists() {
        fs::create_dir(output_path).unwrap_or_else(|_| {
            panic!(
                "failed to create directory: `{}`",
                output_path.to_string_lossy()
            )
        });
    }

    log::info!("output directory: `{}`", output_path.to_string_lossy());

    let config = TrsferServerConfig {
        output_path,
        port,
        allow_port_fallback,
    };

    run_server(config).unwrap();
}

fn run_server(mut config: TrsferServerConfig<'_>) -> io::Result<()> {
    let tcp_listener = loop {
        let addr = format!("127.0.0.1:{}", config.port);
        match TcpListener::bind(&addr) {
            Ok(listener) => {
                log::info!("server is listening on `{}`", addr);
                break listener;
            }
            Err(e) => match e.kind() {
                ErrorKind::AddrInUse => {
                    if !config.allow_port_fallback {
                        exit!(2, config.port);
                    }
                    if let Some(next_port) = config.port.checked_add(1) {
                        config.port = next_port;
                        continue;
                    } else {
                        exit!(3);
                    }
                }
                _ => return Err(e),
            },
        }
    };

    let mut thread_id = 0;
    let output_path = Arc::new(config.output_path.to_owned());

    for stream in tcp_listener.incoming() {
        match stream {
            Ok(stream) => {
                log::info!(
                    "client connected: `{}`",
                    stream.peer_addr().expect("could not connect to server...")
                );

                let thread_builder = Builder::new().name(format!("{}", thread_id));
                let output_path = Arc::clone(&output_path);
                thread_builder
                    .spawn(
                        move || match stream_handler::handle_stream(stream, output_path) {
                            Err(e) if e.kind() != ErrorKind::UnexpectedEof => {
                                log::error!("{}", e);
                            }
                            _ => {}
                        },
                    )
                    .unwrap();
                thread_id += 1;
            }
            Err(e) => {
                log::error!("connection error occured: `{}`", e);
            }
        }
    }

    Ok(())
}
