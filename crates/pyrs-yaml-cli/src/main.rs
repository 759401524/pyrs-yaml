//! Binary shim: all logic lives in the library so benches and tests can
//! drive components directly; clap parses argv and `run` dispatches. The
//! argv may arrive in the jq or yq grammar, which the compat shell
//! translates before clap sees it (see `pyrs_yaml_cli::compat`).

fn main() -> ! {
    pyrs_yaml_cli::main_from_env()
}
