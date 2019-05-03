use failure::Error;
use failure::Fail;

#[derive(Debug, Fail)]
pub enum CeresError {
    #[fail(display = "no main.lua file found")]
    NoMain,
}
