use crate::git::{dir, Dir};
use std::{error::Error, fs::File, io::Write, path::PathBuf};
use structopt::StructOpt;

#[derive(StructOpt)]
pub struct Init {}

pub async fn config_file() -> Result<Option<PathBuf>, Box<dyn Error>> {
    Ok(dir().await?.map(|dir| {
        let Dir { top_level, .. } = dir;
        top_level.join("prefix.yml")
    }))
}

pub async fn init(_: Init) -> Result<(), Box<dyn Error>> {
    if let Some(config) = config_file().await? {
        if !config.exists() {
            File::create(config)?.write_all(sample().as_bytes())?;
        }
    }
    Ok(())
}

fn sample() -> String {
    r#"
# prefix - a git hook manager
#
# git hook names are provided as top level yaml
# keys which contain
# named actions to run
#
# run `prefix install` to install this actions as git hooks
# you can manually invoke these by providing the name of the hook to run `prefix run pre-commit`
pre-commit:
  # short hand notation is just
  # the name of the action followed by a command to run
  test: echo "it works"

  # a more configurable notation
  # is as follows
  #test:
  #  include: *.rs
  #  run: |
  #    echo "it works"
"#
    .into()
}
