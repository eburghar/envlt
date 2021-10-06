use serde_json::Value;
use std::{
	collections::HashMap,
	convert::TryFrom,
	ffi::CString,
	ops::{Deref, DerefMut},
};
use vault_jwt::{client::VaultClient, secret::Secret};

use crate::{
	args::ImportMode,
	error::{Error, Result},
	secret::{Backend, SecretPath},
};

#[derive(Debug)]
pub struct Vars {
	/// Array of CString NAME=VALUE
	vars: Vec<CString>,
	/// Cache of fetched secrets by path
	cache: HashMap<String, Secret>,
}

/// Split a variable definition in NAME and VALUE
fn parse_var(exp: &str) -> Result<(&str, &str)> {
	let i = exp
		.find("=")
		.and_then(|i| if i >= exp.len() { None } else { Some(i) })
		.ok_or_else(|| Error::ParseVar(exp.to_owned()))?;
	Ok((&exp[..i], &exp[i + 1..]))
}

impl Vars {
	pub fn push_secret(
		&mut self,
		prefix: &str,
		secret_path: &SecretPath,
		client: &mut VaultClient,
	) -> Result<()> {
		match secret_path.backend {
			Backend::Vault => {
				let role = secret_path
					.args
					.get(0)
					.ok_or_else(|| Error::MissingRole(format!("{}", &secret_path)))?;
				let method = secret_path
					.args
					.get(1)
					.unwrap_or(&"get")
					.to_ascii_uppercase();

				// get owned secret
				let secret = if let Some(secret) = self.cache.remove(secret_path.path) {
					log::info!("get secret \"{}\" from cache", secret_path.path);
					secret
				} else {
					log::info!("get secret \"{}\" from vault", secret_path.path);
					if !client.is_logged(role) {
						client.login(role)?;
					}
					client.get_secret(
						role,
						&method,
						secret_path.path,
						secret_path.kwargs.as_ref(),
					)?
				};
				// return the value the given pointer or from the root
				let value = if secret_path.anchor != "" {
					secret
						.value
						.pointer(&secret_path.anchor)
						.ok_or_else(|| Error::Pointer(secret_path.anchor.to_owned()))?
				} else {
					&secret.value
				};
				// create variables list from secret
				self.push_value(prefix, value)?;
				// insert the secret (back) into cache
				self.cache.insert(secret_path.path.to_owned(), secret);
			}

			Backend::Const => {
				if let Some(const_type) = secret_path
					.args
					.get(0)
					.filter(|s| **s == "js" || **s == "str")
				{
					if *const_type == "js" {
						let value: Value = serde_json::from_str(secret_path.path)?;
						self.push_value(prefix, &value)?;
					} else {
						self.push(CString::new(format!("{}={}", prefix, secret_path.path))?);
					}
				} else {
					Err(Error::ExpectedArg(
						"\"str\" or \"js\"".to_owned(),
						secret_path.to_string(),
					))?;
				}
			}
		};

		Ok(())
	}

	/// set a list of variable definitions from a list of PREFIX=VAULT_PATH expressions
	/// by fetching the secrets from vault or cache and defining variables from the structure of the secret
	pub fn push_vars(
		&mut self,
		vars: Vec<String>,
		client: &mut VaultClient,
		import_mode: ImportMode,
	) -> Result<()> {
		match import_mode {
			ImportMode::None => {}
			ImportMode::All => {
				for (name, val) in std::env::vars() {
					self.push(CString::new(name + "=" + &val)?);
				}
			}
			ImportMode::OnlyEx | ImportMode::AllEx => {
				for (name, val) in std::env::vars() {
					// if variable value can be parsed to a secretpath
					if let Ok(secret_path) = SecretPath::try_from(val.as_ref()) {
						// push the variables generated by the secret
						self.push_secret(&name, &secret_path, client)?;
					// otherwise push variable as is in AllEx mode
					} else if import_mode == ImportMode::AllEx {
						self.push(CString::new(name + "=" + &val)?);
					}
				}
			}
		}

		// for explicit vault variable, push secret and forward errors
		for var in vars {
			let (prefix, val) = parse_var(&var)?;
			let secret_path = SecretPath::try_from(val.as_ref())?;
			self.push_secret(prefix, &secret_path, client)?;
		}
		Ok(())
	}

	/// Constucts vars from leafs of a parsed json tree
	pub fn push_value(&mut self, key: &str, v: &Value) -> Result<()> {
		match v {
			Value::Null => self.vars.push(CString::new(format!("{}=null", key))?),
			Value::Bool(v) => self
				.vars
				.push(CString::new(format!("{}={}", key, v.to_string()))?),
			Value::Number(v) => self
				.vars
				.push(CString::new(format!("{}={}", key, v.to_string()))?),
			Value::String(v) => self.vars.push(CString::new(format!("{}={}", key, v))?),
			Value::Array(a) => {
				for (i, v) in a.iter().enumerate() {
					self.push_value(&format!("{}_{}", key, i), v)?;
				}
			}
			Value::Object(m) => {
				for (k, v) in m.iter() {
					self.push_value(&format!("{}_{}", key, k.to_ascii_uppercase()), v)?;
				}
			}
		}
		Ok(())
	}
}

impl Default for Vars {
	fn default() -> Self {
		Vars {
			vars: Vec::default(),
			cache: HashMap::default(),
		}
	}
}

impl Deref for Vars {
	type Target = Vec<CString>;

	/// Forwards methods to Vec
	fn deref(&self) -> &Self::Target {
		&self.vars
	}
}

impl DerefMut for Vars {
	/// Forwards methods to Vec
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.vars
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use serde_json::json;

	#[test]
	fn push_value() {
		let value = json!({ "dict": {"key1": "val1", "key2": "val2", "key3": [1, 2, 3, 4]} });
		let mut vars = Vars::default();
		let _ = vars.push_value("VAR", &value).unwrap();
		let vars_str: Vec<&str> = vars
			.iter()
			.map(|s| std::str::from_utf8(s.as_bytes()).unwrap())
			.collect();
		assert_eq!(
			vars_str,
			vec![
				"VAR_DICT_KEY1=val1",
				"VAR_DICT_KEY2=val2",
				"VAR_DICT_KEY3_0=1",
				"VAR_DICT_KEY3_1=2",
				"VAR_DICT_KEY3_2=3",
				"VAR_DICT_KEY3_3=4"
			]
		);
	}

	#[test]
	fn push_value_pointer() {
		let value = json!(
			{ "data": {"key1": "val1", "key2": "val2", "key3": [1, 2, 3, 4]}, "metadata": {"key4": "val4"} }
		);
		let value = value.pointer("/data").unwrap();
		let mut vars = Vars::default();
		let _ = vars.push_value("VAR", &value).unwrap();
		let vars_str: Vec<&str> = vars
			.iter()
			.map(|s| std::str::from_utf8(s.as_bytes()).unwrap())
			.collect();
		assert_eq!(
			vars_str,
			vec![
				"VAR_KEY1=val1",
				"VAR_KEY2=val2",
				"VAR_KEY3_0=1",
				"VAR_KEY3_1=2",
				"VAR_KEY3_2=3",
				"VAR_KEY3_3=4",
			],
		);
	}

	#[test]
	fn push_value_pointer_str() {
		let value = json!({ "data": "val1", "metadata": {"key4": "val4"} });
		let value = value.pointer("/data").unwrap();
		let mut vars = Vars::default();
		let _ = vars.push_value("VAR", &value).unwrap();
		let vars_str: Vec<&str> = vars
			.iter()
			.map(|s| std::str::from_utf8(s.as_bytes()).unwrap())
			.collect();
		assert_eq!(vars_str, vec!["VAR=val1",],);
	}
}
