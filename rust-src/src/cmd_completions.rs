use crate::transform_c_args;
use clap::{Command, CommandFactory, Parser};
use clap_complete::{generate, Generator, Shell};
use std::ffi::{c_char, c_int};
use std::io;

/// Generate shell completions
#[derive(clap::Parser, Debug)]
pub struct Cli {
    shell: Shell,
}

fn print_completions<G: Generator>(gen: G, cmd: &mut Command) {
    generate(gen, cmd, cmd.get_name().to_string(), &mut io::stdout());
}

#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn cmd_completions(argc: c_int, argv: *const *const c_char) -> c_int {
    transform_c_args!(argv, argc, argv);
    let cli = Cli::parse_from(argv);
    print_completions(cli.shell, &mut super::Cli::command());
    0
}
