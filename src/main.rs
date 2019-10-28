//! Automatic maintainability
#![cfg_attr(not(feature = "unstable"), deny(warnings))]
#![deny(missing_docs)]

mod git;
mod init;
mod install;
mod run;
mod uninstall;
use glob::Pattern;
use linked_hash_map::LinkedHashMap;
use serde::Deserialize;
use std::{error::Error, fmt, io};
use structopt::StructOpt;

use init::{init, Init};
use install::{install, Install};
use run::{run, Run};
use uninstall::{uninstall, Uninstall};

#[derive(StructOpt)]
/// a managed githook runner
enum Options {
    /// Intializes git repo with sample config
    Init(Init),
    /// Run hook group
    Run(Run),
    /// Install git hooks
    Install(Install),
    /// Uninstall git hooks
    Uninstall(Uninstall),
}

/// Description of an action to perform on a target st of files
#[derive(Default, Deserialize, Debug, Clone)]
pub struct Action {
    /// name for display
    #[serde(default)]
    name: Option<String>,
    /// pattern of files include
    #[serde(deserialize_with = "deserialize_from_str", default)]
    include: Option<Pattern>,
    /// pattern of files to exclude
    #[serde(deserialize_with = "deserialize_from_str", default)]
    exclude: Option<Pattern>,
    /// command to run (relative to git dir)
    run: String,
}

fn deserialize_from_str<'de, D>(deserializer: D) -> Result<Option<Pattern>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    struct Visitor;

    impl<'de> serde::de::Visitor<'de> for Visitor {
        type Value = Option<Pattern>;

        fn expecting(
            &self,
            formatter: &mut fmt::Formatter<'_>,
        ) -> fmt::Result {
            write!(formatter, "a pattern")
        }

        fn visit_str<E>(
            self,
            v: &str,
        ) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            v.parse().map_err(E::custom).map(Some)
        }
    }

    deserializer.deserialize_str(Visitor)
}

/// Resentations of actions
#[derive(Deserialize, Debug)]
#[serde(untagged)]
pub enum HookDefinition {
    /// A full configuration
    Action(Action),
    /// A simple string for an action
    String(String),
}

impl Into<Action> for HookDefinition {
    fn into(self) -> Action {
        match self {
            HookDefinition::Action(action) => action,
            HookDefinition::String(run) => Action {
                run,
                ..Action::default()
            },
        }
    }
}

/// Ordered mappings of hook name to named action and their definitions
type Config = LinkedHashMap<String, LinkedHashMap<String, HookDefinition>>;

/// Attempt to parse a yaml configuration file
pub fn parse_config<R>(reader: R) -> Result<Config, Box<dyn Error>>
where
    R: io::Read,
{
    Ok(serde_yaml::from_reader::<R, Config>(reader)?)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    match Options::from_args() {
        Options::Init(args) => init(args).await?,
        Options::Run(args) => run(args).await?,
        Options::Install(args) => install(args).await?,
        Options::Uninstall(args) => uninstall(args).await?,
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn hooks_are_parsable() -> Result<(), Box<dyn Error>> {
        let _ = parse_config(&include_bytes!("../tests/data/config.yml")[..])?;
        Ok(())
    }
}
