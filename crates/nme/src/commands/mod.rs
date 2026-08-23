//! One module per subcommand. Each exposes a `run(args) -> ExitCode`-style
//! entry point that `main.rs` dispatches to; the module owns everything
//! about how that command formats output and reports errors.

pub mod info;
