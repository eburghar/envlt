mod args;
mod error;
mod secret;
mod parser_simple;
mod vars;

use anyhow::Result;
use std::{env, ffi::CString, fs::File, io::Read, os::raw::c_char};
use vault_jwt::client::VaultClient;

use crate::{
	args::{Args, ImportMode},
	error::Error,
	vars::Vars,
};

fn main() -> Result<()> {
	// parse command line arguments
	let args: Args = args::from_env();

	// initialize env_logger in info mode for rconfd by default
	env_logger::init_from_env(env_logger::Env::new().default_filter_or("envlt=info"));

	// if token given as argument, get the value from an envar with given name, or just use the string if it fails
	let jwt = if let Some(jwt) = &args.token {
		env::var(jwt).ok().or_else(|| Some(jwt.to_owned())).unwrap()
	// otherwise read from a file
	} else {
		let mut jwt = String::new();
		File::open(&args.token_path)?.read_to_string(&mut jwt)?;
		jwt
	};
	// trim jwt on both ends
	let jwt = jwt.trim();
	let import_mode: ImportMode = (&args).into();

	// initialize a vault client to fetch secret
	let mut client = VaultClient::new(&args.url, &args.login_path, jwt, Some(&args.cacert))?;

	// convert cmd to CString
	let prog = CString::new(args.cmd).unwrap();

	// convert args into CString (move args out of args)
	let mut cmd_args = Vec::<CString>::with_capacity(args.args.len() + 1);
	cmd_args.push(prog.clone());
	for arg in args.args.into_iter() {
		cmd_args.push(CString::new(arg).unwrap())
	}
	// construct a vector of pointers from borrowed iterator (borrowed CString)
	let mut argv: Vec<*const c_char> = cmd_args.iter().map(|s| s.as_ptr()).collect();
	argv.push(std::ptr::null());

	// construct a vec of variables definition from variable expressions (PREFIX=VAULT_PATH)
	let mut env = Vars::default();
	env.push_vars(args.vars, &mut client, import_mode)?;

	// construct a vector of pointers from borrowed iterator (borrowed CString)
	let mut envp: Vec<*const c_char> = env.iter().map(|s| s.as_ptr()).collect();
	envp.push(std::ptr::null());

	// SAFETY: All values pointed by vectors ptr (argv and envp) still exists at this point, so it's safe
	// call execve
	unsafe { libc::execve(prog.as_ptr(), argv.as_ptr(), envp.as_ptr()) };
	Err(Error::ExecError(prog, std::io::Error::last_os_error()))?
}
