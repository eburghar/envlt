mod args;
mod backend;
mod result;
mod vars;

use anyhow::Context;
use std::{env, ffi::CString, fs::File, io::Read, os::raw::c_char};
use vault_jwt::client::VaultClient;

use crate::{
	args::{Args, ImportMode},
	result::Error,
	vars::Vars,
};

fn main() -> anyhow::Result<()> {
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
		File::open(&args.token_path)
			.with_context(|| format!("opening {}", args.token_path))?
			.read_to_string(&mut jwt)
			.with_context(|| format!("reading {}", args.token_path))?;
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
	// construct a vector of pointers from borrowed CString
	let mut argv: Vec<*const c_char> = cmd_args.iter().map(|s| s.as_ptr()).collect();
	argv.push(std::ptr::null());

	// construct a map of variables names, values from expressions (NAME[=VALUE])
	let mut env = Vars::default();
	env.insert_vars(args.vars, &mut client, import_mode)?;
	// transform env back into a vector of NAME=VALUE
	let env = env.get_envp()?;

	// construct a vector of pointers from borrowed CString
	let mut envp: Vec<*const c_char> = env.iter().map(|s| s.as_ptr()).collect();
	envp.push(std::ptr::null());

	// SAFETY: All borrowed values pointed by prt inside argv and envp still exists at this point, so it's safe
	// call execve
	unsafe { libc::execve(prog.as_ptr(), argv.as_ptr(), envp.as_ptr()) };
	Err(Error::ExecError(prog, std::io::Error::last_os_error()))?
}
