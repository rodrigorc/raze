use std::error::Error;
use xshell::{Cmd, cwd, mkdir_p, cp, read_file, write_file};


type Result<T> = std::result::Result<T, Box<dyn Error>>;

fn main() -> Result<()> {
    let mut args = std::env::args();
    args.next(); //skip argv[0]
    let task = args.next();
    return match task.as_deref() {
        None => {
            help()
        }
        Some("pack") => do_pack(&args.collect::<Vec<_>>()),
        Some("deploy") => do_deploy(),
        Some(task) => {
            help()?;
            Err(format!("Unknown xtask '{}'", task).into())
        }
    };
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

fn do_pack(args: &[String]) -> Result<()> {
    let mut mode = "--release";
    for arg in args {
        match arg.as_str() {
            "--debug" => mode = "--debug",
            "--release" => mode = "--release",
            arg => return Err(format!("unknown argument '{}'", arg).into())
        }
    }
    Cmd::new("wasm-pack")
        .arg("build")
        .arg("--no-typescript")
        .arg("--target=web")
        .arg(mode)
        .run()?;
    Ok(())
}

fn do_deploy() -> Result<()> {
    let dst = cwd()?.join("DEMO");
    mkdir_p(&dst)?;
    let pkg = dst.join("pkg");
    mkdir_p(&pkg)?;

    //sed -e '/<head>/r track.html' index.html > DEMO/index.html
    let track = read_file("track.html")?;
    let index = read_file("index.html")?;
    let marker = "<head>\n";
    let pos = index.find(marker).ok_or("<head> not found in index.html")? + marker.len();
    let new_index = String::from(&index[..pos]) + &track + &index[pos..];

    write_file(dst.join("index.html"), new_index)?;

    cp("raze.js", &dst)?;
    cp("raze.css", &dst)?;
    cp("favicon.png", &dst)?;
    cp("base64.js", &dst)?;
    cp("pkg/raze_bg.wasm", &pkg)?;
    cp("pkg/raze.js", &pkg)?;
    println!("Deployed to {:?}! 👍", dst.to_string_lossy());
    Ok(())
}
