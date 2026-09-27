//! Binary shim: all logic lives in the library so benches and tests can
//! drive components directly; clap parses argv and `run` dispatches.

use clap::Parser;

fn main() -> ! {
    pyrs_yaml_cli::run(pyrs_yaml_cli::Cli::parse())
}
