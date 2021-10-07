use std::{convert::TryFrom, fmt};

use crate::error::Error;

/// The different types of supported backend
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum Backend {
	/// Vault
	Vault,
	/// Const
	Const,
}

/// lookup list for backend
const BACKENDS: &'static [(&'static str, Backend)] =
	&[("vault", Backend::Vault), ("const", Backend::Const)];

impl<'a> fmt::Display for Backend {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		for (s, b) in BACKENDS.iter() {
			if self == b {
				return write!(f, "{}", s);
			}
		}
		Ok(())
	}
}

/// Convert a backend text representation into its enum
impl<'a> TryFrom<&'a str> for Backend {
	type Error = Error;

	fn try_from(backend_str: &'a str) -> Result<Self, Self::Error> {
		BACKENDS
			.iter()
			.find_map(|(prefix, backend)| {
				if backend_str.starts_with(*prefix) {
					Some(*backend)
				} else {
					None
				}
			})
			.ok_or(Error::UnknowBackend(backend_str.to_owned()))
	}
}

/// Deserialize a SecretPath
#[derive(PartialEq, Debug)]
pub struct SecretPath<'a> {
    pub backend: Backend,
	pub args: Vec<&'a str>,
	pub kwargs: Option<Vec<(&'a str, &'a str)>>,
	pub path: &'a str,
	pub anchor: &'a str,
}

/// Serialize a SecretPath
impl<'a> fmt::Display for SecretPath<'a> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "{}", self.args.join(","))?;
		if let Some(ref kwargs) = self.kwargs {
			for (k, v) in kwargs.iter() {
				write!(f, ",{}={}", k, v)?;
			}
		}
		write!(f, ":{}", self.path)
	}
}
