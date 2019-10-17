mod git;
mod install;
mod run;
mod uninstall;
use glob::Pattern;
use serde::Deserialize;
use std::{collections::BTreeMap, error::Error, fmt, io};
use structopt::StructOpt;

use install::{install, Install};
use run::{run, Run};
use uninstall::{uninstall, Uninstall};

#[derive(StructOpt)]
/// a managed githook runner
enum Options {
    /// Run hook group
    Run(Run),
    /// Install git hooks
    Install(Install),
    /// Uninstall git hooks
    Uninstall(Uninstall),
}

#[derive(Deserialize, Debug, Clone)]
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

type Config = BTreeMap<String, BTreeMap<String, Action>>;

pub fn parse_config<R>(reader: R) -> Result<Config, Box<dyn Error>>
where
    R: io::Read,
{
    Ok(serde_yaml::from_reader::<R, Config>(reader)?)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    match Options::from_args() {
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
