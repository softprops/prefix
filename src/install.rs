use std::error::Error;
use structopt::StructOpt;

#[derive(StructOpt)]
pub struct Install {}

pub async fn install(_: Install) -> Result<(), Box<dyn Error>> {
    Ok(())
}
