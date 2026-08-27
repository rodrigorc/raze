use clap::{CommandFactory, FromArgMatches, Parser, Subcommand};
use std::error::Error;
use std::path::PathBuf;
use xshell::Shell;

type Result<T> = std::result::Result<T, Box<dyn Error>>;

#[derive(Parser)]
#[command(name = "xtask", about = "Developer task runner")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// 💼 Compile and wasm-pack
    Pack {
        /// Use the debug build profile.
        #[arg(long, conflicts_with = "release")]
        debug: bool,
        /// Use the release build profile (default).
        #[arg(long)]
        release: bool,
    },
    /// 🚀 Update the DEMO worktree
    Deploy {
        /// Destination directory (default: DEMO).
        #[arg(long, default_value = "DEMO")]
        dest: PathBuf,
    },
}

fn main() -> Result<()> {
    let matches = Cli::command().flatten_help(true).get_matches();
    let cli = Cli::from_arg_matches(&matches)?;
    match cli.command {
        Command::Pack { debug, release: _ } => do_pack(debug),
        Command::Deploy { dest } => do_deploy(&dest),
    }
}

fn ch_web(sh: &Shell) {
    sh.change_dir(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../web"));
}

fn do_pack(debug: bool) -> Result<()> {
    let sh = Shell::new()?;
    ch_web(&sh);
    let mode = if debug { "--debug" } else { "--release" };

    sh.cmd("wasm-pack")
        .arg("build")
        .arg("--no-typescript")
        .arg("--target=web")
        .arg(mode)
        .env("CARGO_PROFILE_RELEASE_LTO", "true")
        .env("CARGO_PROFILE_RELEASE_PANIC", "abort")
        .run()?;
    Ok(())
}

fn do_deploy(dest: &PathBuf) -> Result<()> {
    let sh = Shell::new()?;
    ch_web(&sh);
    let dst = sh.current_dir().join(dest);
    sh.create_dir(&dst)?;
    let pkg = dst.join("pkg");
    sh.create_dir(&pkg)?;

    sh.copy_file("index.html", &dst)?;
    sh.copy_file("raze.js", &dst)?;
    sh.copy_file("raze.css", &dst)?;
    sh.copy_file("favicon.png", &dst)?;
    sh.copy_file("base64.js", &dst)?;
    sh.copy_file("pkg/raze_web_bg.wasm", &pkg)?;
    sh.copy_file("pkg/raze_web.js", &pkg)?;
    println!("Deployed to {:?}! 👍", dst.to_string_lossy());
    Ok(())
}
