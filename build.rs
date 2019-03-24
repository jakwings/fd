extern crate clap;

#[path = "src/app.rs"]
mod app;

use std::path::PathBuf;

use clap::Shell;

fn main() {
    let outdir = match std::env::var_os("OUT_DIR") {
        Some(path) => PathBuf::from(path),
        None => PathBuf::from("."),
    };
    let outdir = outdir.join("shell_completion");

    std::fs::create_dir_all(&outdir).unwrap();

    let mut app = self::app::build();
    app.gen_completions("ff", Shell::Bash, &outdir);
    app.gen_completions("ff", Shell::Fish, &outdir);
    app.gen_completions("ff", Shell::Zsh, &outdir);
    app.gen_completions("ff", Shell::PowerShell, &outdir);
}
