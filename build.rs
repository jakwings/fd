extern crate clap;

use clap::Shell;

include!("src/app.rs");

fn main() {
    let outdir = "shell_completion";
    let mut app = build();
    app.gen_completions("ff", Shell::Bash, &outdir);
    app.gen_completions("ff", Shell::Fish, &outdir);
    app.gen_completions("ff", Shell::Zsh, &outdir);
    app.gen_completions("ff", Shell::PowerShell, &outdir);
}
