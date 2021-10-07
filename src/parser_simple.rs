use crate::{
	error::{Error, Result},
	secret::{Backend, SecretPath},
};

use std::convert::TryFrom;

/// States of finite state machine for parsing secretpath
enum Pos {
	Backend,
	Args,
	Path,
	Anchor,
}

/// Iterator, that returns the 3 successives slices separated by a colon from an expression
/// backend:args:path. backend and args can't contain ':' and there is no escaping mechanism
pub struct SecretPathIterator<'a> {
	remainder: &'a str,
	pos: Pos,
}

impl<'a> SecretPathIterator<'a> {
	pub fn new(s: &'a str) -> Self {
		Self {
			remainder: s,
			pos: Pos::Backend,
		}
	}

	/// simply return remainder
	pub fn yield_remainder(&mut self) -> Option<&'a str> {
		let remainder = self.remainder;
		self.remainder = "";
		Some(remainder)
	}

	/// returns the slice up to ':' and advances after the ':'
	pub fn yield_colon(&mut self) -> Option<&'a str> {
		match self.remainder.find(":") {
			Some(pos) => {
				let res = &self.remainder[..pos];
				self.remainder = if pos + 1 < self.remainder.len() {
					&self.remainder[pos + 1..]
				} else {
					""
				};
				Some(res)
			}
			None => None,
		}
	}

	/// returns the slice up to '#' or return the remainder
	pub fn yield_colon_hash(&mut self) -> Option<&'a str> {
		match self.remainder.find("#") {
			Some(pos) => {
				let res = &self.remainder[..pos];
				self.remainder = if pos + 1 < self.remainder.len() {
					&self.remainder[pos + 1..]
				} else {
					""
				};
				Some(res)
			}
			None => self.yield_remainder(),
		}
	}
}

impl<'a> Iterator for SecretPathIterator<'a> {
	type Item = &'a str;

	fn next(&mut self) -> Option<Self::Item> {
		if self.remainder.is_empty() {
			None
		} else {
			match self.pos {
				Pos::Backend => {
					self.pos = Pos::Args;
					self.yield_colon()
				}
				Pos::Args => {
					self.pos = Pos::Path;
					self.yield_colon()
				}
				Pos::Path => {
					self.pos = Pos::Anchor;
					self.yield_colon_hash()
				}
				Pos::Anchor => self.yield_remainder(),
			}
		}
	}
}

/// Simple SecretPath parser: backend:arg_1(,arg_n)*(,key_n=val_n):path:jsonpointer
impl<'a> TryFrom<&'a str> for SecretPath<'a> {
	type Error = Error;

	fn try_from(path: &'a str) -> Result<Self> {
		// split all path components
		let mut it = SecretPathIterator::new(path);
		let backend_str = it.next().ok_or(Error::NoBackend)?;
		let backend = Backend::try_from(backend_str)?;
		let args_ = it.next().ok_or(Error::NoArgs(path.to_owned()))?;
		let path = it.next().ok_or(Error::NoPath(args_.to_owned()))?;
		let anchor = it.next().unwrap_or("");
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
			backend,
			args,
			kwargs: if kwargs.is_empty() {
				None
			} else {
				Some(kwargs)
			},
			path,
			anchor,
		})
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn parse_anchor() {
		let path = "vault:role,POST,common_name=example.com:pki/issue/example.com#/data";
		let secret_path = SecretPath::try_from(path).unwrap();
		assert_eq!(
			secret_path,
			SecretPath {
				backend: Backend::Vault,
				args: vec!["role", "POST"],
				kwargs: Some(vec![("common_name", "example.com")]),
				path: "pki/issue/example.com",
				anchor: "/data"
			}
		);
	}

	#[test]
	fn parse_const_str() {
		let path = "const:str:https://localhost:8200";
		let secret_path = SecretPath::try_from(path).unwrap();
		assert_eq!(
			secret_path,
			SecretPath {
				backend: Backend::Const,
				args: vec!["str"],
				kwargs: None,
				path: "https://localhost:8200",
				anchor: ""
			}
		);
	}

	#[test]
	fn parse_const_json() {
		let path = r#"const:js:{"key": "val"}"#;
		let secret_path = SecretPath::try_from(path).unwrap();
		assert_eq!(
			secret_path,
			SecretPath {
				backend: Backend::Const,
				args: vec!["js"],
				kwargs: None,
				path: r#"{"key": "val"}"#,
				anchor: ""
			}
		);
	}
}
