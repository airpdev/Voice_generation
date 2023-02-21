extern crate hound;
extern crate csv;
use uuid::Uuid;
use std::process::{Command};
use std::io;
use crate::models::voice_generation::{
    Silence,
};
use std::time::{SystemTime, UNIX_EPOCH};
use std::fs::{self, File};
use std::io::prelude::*;
use std::io::LineWriter;
use std::env;
use csv::Writer;
use std::fs::OpenOptions;

pub fn launch_lipsync_generate(param : String) {
	println!("launch_lipsync started =====================");

	let mut child = Command::new("bash")
							.arg("lipsync_generate.sh")
							.arg(param)
							.spawn()
							.expect("Failed to launch_lipsync process");
	child.wait().expect("failed to wait on processing launch_lipsync");

	println!("launch_lipsync finished =====================");
}
