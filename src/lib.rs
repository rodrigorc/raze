#![allow(unused)]
#![warn(unreachable_patterns)]

use std::env;
use std::io::{self, BufWriter, Write, ErrorKind};
use std::fs::File;
use std::path::Path;

#[macro_use]
pub mod js;
mod game;
mod memory;
mod z80;
mod tape;
