#![allow(dead_code,unexpected_cfgs)]
include!("../../shared.rs");
use pinocchio::{error::ProgramError,Address,ProgramResult};
#[cfg(feature="benchmark")]
include!("benchmark.rs");
#[cfg(not(feature="benchmark"))]
mod game;
