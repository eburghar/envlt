use argh::{FromArgs, TopLevelCommand};
use std::{convert::From, env, path::Path};

/// Get vault secrets from path expressions, define environment variables, then execute into args and command
#[derive(FromArgs)]
pub struct Args {
	/// the vault url ($VAULT_URL or https://localhost:8200/v1)
	#[argh(option, short = 'u', default = "default_url()")]
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

	/// an expression NAME[=VALUE] for defining one or several variables. When no VALUE given, an environment variable
	/// with the same name is imported, when VALUE doesn't match a expression, a new variable is defined with the provided
	/// VALUE, otherwise the expression is expanded in one or several variables and NAME is used as a prefix.
	#[argh(option, short = 'V')]
	pub vars: Vec<String>,

	/// verbose mode
	#[argh(switch, short = 'v')]
	pub verbose: bool,

	/// import all environment variables before executing into cmd
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

/// returns the default vault url if not defined on command line argument
/// VAULT_URL or localhost if undefined
fn default_url() -> String {
	env::var("VAULT_URL")
		.ok()
		.or_else(|| Some("https://localhost:8200/v1".to_owned()))
		.unwrap()
}

/// Express all -i and -I combinations
#[derive(PartialEq)]
pub enum ImportMode {
	// only defined variables (with -V) are imported (no -i, nor -I)
	None,
	// all environment variables are imported as is (-i but no -I)
	All,
	// only environment variables matching a vault_path and successfuly expanded are imported (-I but no -i)
	OnlyEx,
	// all environment variables are imported and expanded whenever possible (-i and -I)
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

/// copy of argh::from_env to insert command name and version
pub fn from_env<T: TopLevelCommand>() -> T {
	const NAME: &'static str = env!("CARGO_BIN_NAME");
	const VERSION: &'static str = env!("CARGO_PKG_VERSION");
	let args: Vec<String> = std::env::args().collect();
	// get the file name of path or the full path
	let cmd = Path::new(&args[0])
		.file_name()
		.map_or(None, |s| s.to_str())
		.unwrap_or(&args[0]);
	let args_str: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
	T::from_args(&[cmd], &args_str[1..]).unwrap_or_else(|early_exit| {
		println!("{} {}\n", NAME, VERSION);
		println!("{}", early_exit.output);
		std::process::exit(match early_exit.status {
			Ok(()) => 0,
			Err(()) => 1,
		})
	})
}
