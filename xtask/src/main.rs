use clap::{CommandFactory, FromArgMatches, Parser, Subcommand};
use lightningcss::stylesheet::{ParserOptions, PrinterOptions, StyleSheet};
use lightningcss::targets::Browsers;
use std::error::Error;
use std::path::{Path, PathBuf};
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
    let mode = if debug { "--dev" } else { "--release" };

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

fn compile_css(sh: &Shell, src: &str, dst: &Path) -> Result<()> {
    let cd = sh.current_dir();

    let src = cd.join(src);
    let dst = cd.join(dst).join(src.file_name().unwrap());

    let modern_css = std::fs::read_to_string(&src)?;

    let stylesheet = StyleSheet::parse(&modern_css, ParserOptions::default())
        .map_err(|e| format!("CSS parse error: {e}"))?;
    let mut popts = PrinterOptions::default();
    popts.targets.browsers = Some(Browsers {
        chrome: Some(57),
        firefox: Some(52),
        safari: Some(11),
        edge: Some(16),
        ..Browsers::default()
    });
    let old_css = stylesheet
        .to_css(popts)
        .map_err(|e| format!("CSS write error: {e}"))?;

    std::fs::write(dst, old_css.code)?;
    Ok(())
}

fn copy_file(sh: &Shell, src: &str, dst: &Path, patch_version: Option<&str>) -> Result<()> {
    match patch_version {
        None => {
            sh.copy_file(src, dst)?;
        }
        Some(ver) => {
            let cd = sh.current_dir();

            let src = cd.join(src);
            let dst = cd.join(dst).join(src.file_name().unwrap());

            let text = std::fs::read_to_string(&src)?;
            let patched = text.replace("__VERSION__", ver);
            std::fs::write(&dst, &patched)?;
        }
    }
    Ok(())
}

fn do_deploy(dest: &Path) -> Result<()> {
    let sh = Shell::new()?;
    ch_web(&sh);
    let dst = sh.current_dir().join(dest);
    sh.create_dir(&dst)?;
    let pkg = dst.join("pkg");
    sh.create_dir(&pkg)?;

    let ver = sh
        .cmd("git")
        .arg("rev-parse")
        .arg("--short")
        .arg("HEAD")
        .read()?;
    let ver = &ver;

    copy_file(&sh, "index.html", &dst, Some(ver))?;
    copy_file(&sh, "raze.js", &dst, Some(ver))?;
    compile_css(&sh, "raze.css", &dst)?;
    copy_file(&sh, "favicon.png", &dst, None)?;
    copy_file(&sh, "base64.js", &dst, Some(ver))?;
    copy_file(&sh, "pkg/raze_web_bg.wasm", &pkg, None)?;
    copy_file(&sh, "pkg/raze_web.js", &pkg, Some(ver))?;
    println!("Deployed to {:?}! 👍", dst.to_string_lossy());
    Ok(())
}
