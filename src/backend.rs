use std::{convert::TryFrom, fmt};
use vault_jwt::error::Error;

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
