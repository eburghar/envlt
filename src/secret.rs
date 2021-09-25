use std::{convert::TryFrom, fmt};

/// CustomError enum for clear error messages
#[derive(Debug, PartialEq)]
pub enum Error {
	NoArgs(String),
	NoPath(String),
	ExtraData(String),
}

impl std::error::Error for Error {}

/// Proper display of errors
impl fmt::Display for Error {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match self {
			Error::NoArgs(s) => {
				write!(
					f,
					"missing a \":\" to separate backend and arguments somewhere in \"{}\"",
					s
				)
			}
			Error::NoPath(s) => write!(
				f,
				"missing a \":\" to separate arguments and path somewhere in \"{}\"",
				s
			),
			Error::ExtraData(s) => write!(f, "extra data after path \"{}\"", s),
		}
	}
}

/// Deserialize a secret path
pub struct SecretPath<'a> {
	pub args: Vec<&'a str>,
	pub kwargs: Option<Vec<(&'a str, &'a str)>>,
	pub path: &'a str,
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

/// Simple secret path parser: arg1[,args][,key1=val1]:path
impl<'a> TryFrom<&'a str> for SecretPath<'a> {
	type Error = Error;

	fn try_from(path: &'a str) -> Result<Self, Self::Error> {
		// split all path components
		let mut it = path.split(":");
		let args_ = it.next().ok_or(Error::NoArgs(path.to_owned()))?;
		let path = it.next().ok_or(Error::NoPath(args_.to_owned()))?;
		if it.next().is_some() {
			Err(Error::ExtraData(path.to_owned()))?;
		}
		// split simple and keyword arguments in separate lists
		let mut args = Vec::with_capacity(args_.len());
		let mut kwargs = Vec::with_capacity(args_.len());
		for arg in args_.split(",") {
			if let Some(pos) = arg.find('=') {
				kwargs.push((&arg[..pos], &arg[pos + 1..]));
			} else {
				args.push(arg);
			}
		}

		Ok(Self {
			args,
			kwargs: if kwargs.is_empty() {
				None
			} else {
				Some(kwargs)
			},
			path,
		})
	}
}
