// SPDX-License-Identifier: GPL-3.0-only

mod create;
mod util;

use std::error::Error;
use std::ffi::OsStr;
use std::iter;

use clap::Parser;

use crate::util::crun;

// Adapted from https://github.com/containers/youki/blob/main/crates/youki/src/main.rs
#[derive(Parser, Debug)]
struct Args {
    #[clap(flatten)]
    global: liboci_cli::GlobalOpts,

    #[clap(subcommand)]
    command: Command,
}

// Adapted from https://github.com/containers/youki/blob/main/crates/youki/src/main.rs
#[derive(clap::Parser, Debug)]
enum Command {
    #[clap(flatten)]
    Standard(Box<liboci_cli::StandardCmd>),

    #[clap(flatten)]
    Common(Box<liboci_cli::CommonCmd>),
}

pub fn main<I, S>(args: I) -> Result<(), Box<dyn Error>>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let args = args
        .into_iter()
        .map(|a| a.as_ref().to_os_string())
        .collect::<Vec<_>>();

    let parsed_args =
        Args::parse_from(iter::once(&OsStr::new("crun-qemu").to_os_string()).chain(&args));

    if let Command::Standard(cmd) = parsed_args.command {
        if let liboci_cli::StandardCmd::Create(create_args) = *cmd {
            return create::create(&parsed_args.global, &create_args);
        }
    }

    // not a command we implement ourselves, just pass it on to crun
    crun(&args)?;
    Ok(())
}