use crate::{
    git,
    git::{dir, Dir},
};
use std::{error::Error, io, path::PathBuf};
use structopt::StructOpt;

#[derive(StructOpt)]
pub struct Uninstall {}

fn remove_hook(hook: PathBuf) -> io::Result<()> {
    if hook.exists() {
        println!("removing hook {}", hook.display());
        std::fs::remove_file(hook)?
    }
    Ok(())
}

pub async fn uninstall(_: Uninstall) -> Result<(), Box<dyn Error>> {
    if let Some(Dir { git_dir, .. }) = dir().await? {
        let hooks_dir = git_dir.join("hooks");
        if hooks_dir.exists() {
            for hook in git::HOOKS.iter().map(|hook| hooks_dir.join(hook)) {
                remove_hook(hook)?;
            }
        }
    }
    Ok(())
}
