// https://users.rust-lang.org/t/rookie-going-from-std-process-to-libc-exec/10180/2
mod args;

use std::{ffi::CString, os::raw::c_char};

use crate::args::Args;

fn main() {
	// parse command line arguments
	let args: Args = args::from_env();

	// convert cmd to pointer
	let cmd = CString::new(args.cmd).unwrap();

	// convert cmd_args to array of pointers
	let cmd_args: Vec<CString> = args.args
		.into_iter()
		.map(|s| CString::new(s).unwrap())
		.collect();
	let mut cmd_args_raw: Vec<*const c_char> = cmd_args.iter().map(|s| s.as_ptr()).collect();
	cmd_args_raw.push(std::ptr::null());

	// convert env to array of pointers
	let env: Vec<CString> = args.vars
		.into_iter()
		.map(|s| CString::new(s).unwrap())
		.collect();
	let mut env_raw: Vec<*const c_char> = env.iter().map(|s| s.as_ptr()).collect();
	env_raw.push(std::ptr::null());

	// execute into given command and args with given env vars
	unsafe { libc::execve(cmd.as_ptr(), cmd_args_raw.as_ptr(), env_raw.as_ptr()) };
}
