use argh::{FromArgs, TopLevelCommand};
use std::path::Path;

/// Get vault secrets from path, modify environment, then executure args and command
#[derive(FromArgs)]
pub struct Args {
	/// the vault url (https://localhost:8200)
	#[argh(
		option,
		short = 'u',
		default = "\"https://localhost:8200/v1\".to_owned()"
	)]
	pub url: String,

	/// the env variable containing the JWT token (CI_JOB_JWT)
	#[argh(option, short = 'j', default = "\"CI_JOB_JWT\".to_owned()")]
	pub jwt: String,

	/// the login path (/auth/jwt/login)
	#[argh(
		option,
		short = 'l',
		default = "\"/auth/jwt/login\".to_owned()"
	)]
	pub login_path: String,

	/// an expression NAME=PATH for defining a variable named NAME from a vault path expression
	#[argh(option, short = 'v')]
	pub vars: Vec<String>,

	/// command to execute into
	#[argh(positional)]
	pub cmd: String,

	// arguments of command
	#[argh(positional)]
	pub args: Vec<String>,
}

fn cmd<'a>(default: &'a String, path: &'a String) -> &'a str {
	Path::new(path)
		.file_name()
		.map(|s| s.to_str())
		.flatten()
		.unwrap_or(default.as_str())
}

/// copy of argh::from_env to insert command name and version
pub fn from_env<T: TopLevelCommand>() -> T {
	const NAME: &'static str = env!("CARGO_PKG_NAME");
	const VERSION: &'static str = env!("CARGO_PKG_VERSION");
	let strings: Vec<String> = std::env::args().collect();
	let cmd = cmd(&strings[0], &strings[0]);
	let strs: Vec<&str> = strings.iter().map(|s| s.as_str()).collect();
	T::from_args(&[cmd], &strs[1..]).unwrap_or_else(|early_exit| {
		println!("{} {}\n", NAME, VERSION);
		println!("{}", early_exit.output);
		std::process::exit(match early_exit.status {
			Ok(()) => 0,
			Err(()) => 1,
		})
	})
}
