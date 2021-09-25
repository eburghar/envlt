// https://users.rust-lang.org/t/rookie-going-from-std-process-to-libc-exec/10180/2
mod args;
mod secret;

use anyhow::{anyhow, Context, Result};
use std::{convert::TryFrom, env, ffi::CString, os::raw::c_char};
use vaultk8s::client::VaultClient;

use crate::{args::Args, secret::SecretPath};

fn parse_var(exp: &str) -> Result<(&str, &str)> {
	let i = exp
		.find("=")
		.and_then(|i| if i >= exp.len() { None } else { Some(i) })
		.ok_or_else(|| anyhow!("variable expression should be name=value: \"{}\"", exp))?;
	Ok((&exp[..i], &exp[i + 1..]))
}

fn main() -> Result<()> {
	// parse command line arguments
	let args: Args = args::from_env();

	let jwt = env::var(&args.jwt).with_context(|| {
		format!(
			"failed to get jwt token from environment variable {}",
			&args.jwt
		)
	})?;
	let mut client = VaultClient::new(&args.url, &args.login_path, &jwt, None)?;

	// convert cmd to pointer
	let cmd = CString::new(args.cmd).unwrap();

	// convert cmd_args to array of pointers
	let cmd_args: Vec<CString> = args
		.args
		.into_iter()
		.map(|s| CString::new(s).unwrap())
		.collect();
	let mut cmd_args_raw: Vec<*const c_char> = cmd_args.iter().map(|s| s.as_ptr()).collect();
	cmd_args_raw.push(std::ptr::null());

	// convert env to array of pointers
	let mut env: Vec<CString> = Vec::with_capacity(args.vars.len());
	for var in args.vars {
		let (name, path) = parse_var(&var)?;
		let secret_path = SecretPath::try_from(path)?;
		let role = secret_path.args[0];
		let method = secret_path.args.get(1).unwrap_or(&"get").to_ascii_uppercase();
		if !client.is_logged(role) {
			client
				.login(role)?;
		}
		let secret = client.get_secret(role, &method, secret_path.path, secret_path.kwargs.as_ref())?;
		env.push(CString::new(format!("{}={}", name, &secret.value)).unwrap());
	}
	let mut env_raw: Vec<*const c_char> = env.iter().map(|s| s.as_ptr()).collect();
	env_raw.push(std::ptr::null());

	// execute into given command and args with given env vars in context
	unsafe { libc::execve(cmd.as_ptr(), cmd_args_raw.as_ptr(), env_raw.as_ptr()) };
	Ok(())
}
