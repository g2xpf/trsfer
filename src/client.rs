use std::{net::SocketAddr, path::Path};

use clap::ArgMatches;

use super::{Error, Result};

pub mod session;
pub mod thread_pool;

use session::Session;

struct TrsferClientConfig<'a> {
    addr: SocketAddr,
    path: &'a Path,
    is_dry_run: bool,
}

pub fn run(matches: &ArgMatches<'_>) -> Result<()> {
    let ip = matches.value_of("ip").unwrap();

    let is_dry_run = matches.is_present("dry run");

    let port = match matches.value_of("port") {
        Some(port) => match port.parse::<u16>() {
            Ok(port) => port,
            _ => exit!(1, port),
        },
        _ => unreachable!(),
    };

    let path = matches.value_of("path").unwrap();
    let path = Path::new(path);

    let addr_str = format!("{}:{}", ip, port);
    let addr = match addr_str.parse() {
        Ok(addr) => addr,
        _ => exit!(6, addr_str),
    };

    let config = TrsferClientConfig {
        addr,
        path,
        is_dry_run,
    };

    handle_stream(&config)
}

fn handle_stream(config: &TrsferClientConfig) -> Result<()> {
    let num_sessions = num_cpus::get();
    let session = Session::connect(num_sessions, config.addr).map_err(Error::IOError)?;
    session.run_by_path(config.path, config.is_dry_run)
}
