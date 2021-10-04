use argh::{FromArgs, TopLevelCommand};
use std::{convert::From, path::Path};

/// Get vault secrets from path, modify environment, then executure args and command
#[derive(FromArgs)]
pub struct Args {
	/// the vault url ($VAULT_URL or https://localhost:8200/v1)
	#[argh(
		option,
		short = 'u',
		default = "\"https://localhost:8200/v1\".to_owned()"
	)]
	pub url: String,

	/// the login path (/auth/jwt/login)
	#[argh(option, short = 'l', default = "\"/auth/jwt/login\".to_owned()")]
	pub login_path: String,

	/// path of vault CA certificate (/var/run/secrets/kubernetes.io/serviceaccount/ca.crt)
	#[argh(
		option,
		short = 'c',
		default = "\"/var/run/secrets/kubernetes.io/serviceaccount/ca.crt\".to_owned()"
	)]
	pub cacert: String,

	/// the JWT token taken from the given variable name or from the given string if it fails (take precedence over -t)
	#[argh(option, short = 'T')]
	pub token: Option<String>,

	/// path of the JWT token (/var/run/secrets/kubernetes.io/serviceaccount/token)
	#[argh(
		option,
		short = 't',
		default = "\"/var/run/secrets/kubernetes.io/serviceaccount/token\".to_owned()"
	)]
	pub token_path: String,

	/// an expression NAME=PATH for defining a variable named NAME from a vault path expression
	#[argh(option, short = 'V')]
	pub vars: Vec<String>,

	/// verbose mode
	#[argh(switch, short = 'v')]
	pub verbose: bool,

	/// import all environment variables before executing cmd
	#[argh(switch, short = 'i')]
	pub import: bool,

	/// import environment variables whose values matches a vault_path a whose expansion is successful
	#[argh(switch, short = 'I')]
	pub import_vault: bool,

	/// command to execute into
	#[argh(positional)]
	pub cmd: String,

	// arguments of command
	#[argh(positional)]
	pub args: Vec<String>,
}

/// Uniformize -i and -I combinations into an enum
pub enum ImportMode {
	// no -i, or -I: only defined variables (-V) imported
	None,
	// -i and no -I: all environment variables are imported as is
	All,
	// -I and no -i: only environment variable matching a vault_path and successfuly expanded are imported
	OnlyEx,
	// all environment variables are imported and expanded using vault_path whenever possible
	AllEx,
}

impl From<&Args> for ImportMode {
	fn from(args: &Args) -> ImportMode {
		if args.import_vault {
			if args.import {
    			ImportMode::AllEx
			} else {
    			ImportMode::OnlyEx
			}
		} else if args.import {
    		if args.import_vault {
        		ImportMode::AllEx
    		} else {
    			ImportMode::All
    		}
		} else {
			ImportMode::None
		}
	}
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
	const NAME: &'static str = env!("CARGO_BIN_NAME");
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
