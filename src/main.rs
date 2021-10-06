use std::io::Write;
use std::thread;

use clap::{App, AppSettings, Arg, SubCommand};

use env_logger::Builder;

use trsfer::{client, server, DEFAULT_IP_ADDRESS, DEFAULT_PORT};

const VERSION: &str = env!("CARGO_PKG_VERSION");
const AUTHOR: &str = env!("CARGO_PKG_AUTHORS");
const ABOUT: &str = r#"trsfer is a command line tool that transfers files through a local network"#;

fn main() {
    let mut builder = Builder::from_default_env();
    builder
        .format(|buf, record| {
            let thread = thread::current();
            let thread_name = thread.name().unwrap_or("");
            let time = buf.timestamp_millis();
            let level = record.level();
            let level = buf.default_styled_level(level);
            let file = record.file().unwrap_or("?");
            let line = match record.line() {
                Some(line) => line.to_string(),
                None => String::from("?"),
            };
            let args = record.args();
            writeln!(
                buf,
                "[{time} {level} [{thread_name}] {file}:{line}] {args}",
                time = time,
                level = level,
                thread_name = thread_name,
                file = file,
                line = line,
                args = args
            )
        })
        .init();

    let default_port_str = &format!("{}", DEFAULT_PORT);
    let matches = App::new("trsfer")
        .version(VERSION)
        .author(AUTHOR)
        .about(ABOUT)
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .subcommand(
            SubCommand::with_name("server")
                .about("Launch a trsfer server")
                .arg(
                    Arg::from_usage(
                        "[ip] -a, --ip-address=<ip> 'the ip address that the server listens on'",
                    )
                    .default_value(DEFAULT_IP_ADDRESS),
                )
                .arg(
                    Arg::from_usage(
                        "[port] -p, --port=<PORT> 'the port that the server listens on'",
                    )
                    .default_value(&default_port_str),
                )
                .arg_from_usage("[output] -o, --output=<OUTPUT> 'output directory'"),
        )
        .subcommand(
            SubCommand::with_name("client")
                .about("Launch a trsfer client and connect to the trsfer server")
                .arg(
                    Arg::from_usage(
                        "[port] -p, --port=<PORT> 'the port the server is listening on'",
                    )
                    .default_value(&default_port_str),
                )
                .arg_from_usage("[dry run] -d, --dry-run 'dry-run (only shows log messages)'")
                .arg_from_usage("<ip> 'the server ip address'")
                .arg_from_usage("<path> 'copy path (file/directory)'"),
        )
        .get_matches();

    if let Some(matches) = matches.subcommand_matches("server") {
        server(matches);
    } else if let Some(matches) = matches.subcommand_matches("client") {
        client(matches);
    } else {
        unreachable!();
    }
}
