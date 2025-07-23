use std::error::Error;
use std::path::PathBuf;
use xshell::Shell;

type Result<T> = std::result::Result<T, Box<dyn Error>>;

fn main() -> Result<()> {
    let mut args = std::env::args();
    args.next(); //skip argv[0]
    let task = args.next();
    match task.as_deref() {
        None => help(),
        Some("pack") => do_pack(&args.collect::<Vec<_>>()),
        Some("deploy") => do_deploy(),
        Some(task) => {
            help()?;
            Err(format!("Unknown xtask '{task}'").into())
        }
    }
}

fn help() -> Result<()> {
    println!("USAGE: ");
    println!("    xtask [TASK]");
    println!();
    println!("Available TASKs:");
    println!("    pack      💼 Compile and wasm-pack. Can Add --debug or --release (default).");
    println!("    deploy    🚀 Update the DEMO worktree");
    println!();
    Ok(())
}

fn ch_web(sh: &Shell) {
    sh.change_dir(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../web"));
}

fn do_pack(args: &[String]) -> Result<()> {
    let sh = Shell::new()?;
    ch_web(&sh);
    let mut mode = "--profile=web";
    for arg in args {
        match arg.as_str() {
            "--debug" => mode = "--debug",
            "--release" => {}
            arg => return Err(format!("unknown argument '{arg}'").into()),
        }
    }
    sh.cmd("wasm-pack")
        .arg("build")
        .arg("--no-typescript")
        .arg("--target=web")
        .arg(mode)
        .run()?;
    Ok(())
}

fn do_deploy() -> Result<()> {
    let sh = Shell::new()?;
    ch_web(&sh);
    let dst = sh.current_dir().join("DEMO");
    sh.create_dir(&dst)?;
    let pkg = dst.join("pkg");
    sh.create_dir(&pkg)?;

    sh.copy_file("index.html", &dst)?;
    sh.copy_file("raze.js", &dst)?;
    sh.copy_file("raze.css", &dst)?;
    sh.copy_file("favicon.png", &dst)?;
    sh.copy_file("base64.js", &dst)?;
    sh.copy_file("pkg/raze_bg.wasm", &pkg)?;
    sh.copy_file("pkg/raze.js", &pkg)?;
    println!("Deployed to {:?}! 👍", dst.to_string_lossy());
    Ok(())
}
