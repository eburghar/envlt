use serde_json::Value;
use std::{
	collections::{hash_map, HashMap},
	convert::TryFrom,
	env,
	ffi::CString,
	ops::{Deref, DerefMut},
};
use vault_jwt::{
	client::VaultClient,
	secret::{Secret, SecretPath},
};

use crate::{
	args::ImportMode,
	backend::Backend,
	result::{Error, Result},
};

#[derive(Debug, Default)]
pub struct Vars {
	/// Array of CString NAME=VALUE
	vars: HashMap<String, String>,
	/// Cache of fetched secrets by path
	cache: HashMap<String, Secret>,
}

/// Split a variable definition in NAME and VALUE
fn parse_var(exp: &str) -> Result<(&str, Option<&str>)> {
	let i = exp.find('=');
	if let Some(i) = i {
		// something after =
		if i + 1 < exp.len() {
			Ok((&exp[..i], Some(&exp[i + 1..])))
		// nothing after =
		} else {
			Ok((&exp[..i], Some("")))
		}
	} else {
		Ok((exp, None))
	}
}

impl Vars {
	/// insert one or several variables from a path
	pub fn insert_path(
		&mut self,
		prefix: &str,
		secret_path: &SecretPath<Backend>,
		client: &mut VaultClient,
	) -> Result<()> {
		match secret_path.backend {
			Backend::Vault => {
				let role = secret_path
					.args
					.get(0)
					.ok_or_else(|| Error::MissingRole(format!("{}", secret_path)))?;
				let method = secret_path
					.args
					.get(1)
					.unwrap_or(&"get")
					.to_ascii_uppercase();

				// get owned secret
				let secret = if let Some(secret) = self.cache.remove(secret_path.path) {
					log::info!(
						"get \"{}\" as {} from cache",
						secret_path.to_string(),
						prefix
					);
					secret
				} else {
					log::info!("get \"{}\" as {}", secret_path.to_string(), prefix);
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
				// return the value from the given pointer or from the root
				let value = if let Some(anchor) = secret_path.anchor {
					secret
						.value
						.pointer(anchor)
						.ok_or_else(|| Error::Pointer(anchor.to_owned()))?
				} else {
					&secret.value
				};
				// create variables list from secret
				self.insert_value(prefix, value)?;
				// insert the secret (back) into cache
				self.cache.insert(secret_path.path.to_owned(), secret);
			}

			Backend::Const => {
				if let Some(const_type) = secret_path
					.args
					.get(0)
					.filter(|s| **s == "js" || **s == "str")
				{
					log::info!("get \"{}\" as {}", secret_path.to_string(), prefix);
					if *const_type == "js" {
						let value: Value = serde_json::from_str(secret_path.full_path)
							.map_err(|e| Error::Parse(secret_path.full_path.to_owned(), e))?;
						self.insert_value(prefix, &value)?;
					} else {
						self.insert(prefix.to_owned(), secret_path.full_path.to_owned());
					}
				} else {
					return Err(Error::ExpectedArg(
						"\"str\" or \"js\"".to_owned(),
						secret_path.to_string(),
					));
				}
			}
		};

		Ok(())
	}

	/// insert a list of variable definitions from a list of string NAME[=VALUE]
	/// fetching the secrets from backend if necessary
	pub fn insert_vars(
		&mut self,
		vars: Vec<String>,
		client: &mut VaultClient,
		import_mode: ImportMode,
	) -> Result<()> {
		match import_mode {
			ImportMode::None => {}
			ImportMode::All => {
				for (name, val) in std::env::vars() {
					self.insert(name, val);
				}
			}
			ImportMode::OnlyEx | ImportMode::AllEx => {
				for (name, val) in std::env::vars() {
					// if variable value can be parsed to a secretpath
					if let Ok(secret_path) = SecretPath::try_from(val.as_ref()) {
						// push the variables generated by the secret
						self.insert_path(&name, &secret_path, client)?;
					// otherwise push variable as is in AllEx mode
					} else if import_mode == ImportMode::AllEx {
						self.insert(name, val);
					}
				}
			}
		}

		// for explicit variable import, push secret or env var value and forward errors
		for var in vars {
			let (prefix, val) = parse_var(&var)?;
			// if val is not defined, try to get it from the environment
			let val = val.map(|s| s.to_owned()).or_else(|| env::var(prefix).ok());
			// if we have a name and a value
			if let Some(val) = val {
				// try to parse the value as a secret path and push
				if let Ok(secret_path) = SecretPath::try_from(val.as_ref()) {
					self.insert_path(prefix, &secret_path, client)?;
				// otherwise push the var as is
				} else {
					self.insert(prefix.to_owned(), val);
				}
			}
		}
		Ok(())
	}

	/// Constucts vars from leafs of a parsed json tree
	pub fn insert_value(&mut self, key: &str, v: &Value) -> Result<()> {
		match v {
			Value::Null => {
				self.vars.insert(key.to_owned(), "null".to_owned());
			}
			Value::Bool(v) => {
				self.vars.insert(key.to_owned(), v.to_string());
			}
			Value::Number(v) => {
				self.vars.insert(key.to_owned(), v.to_string());
			}
			Value::String(v) => {
				self.vars.insert(key.to_owned(), v.to_owned());
			}
			Value::Array(a) => {
				for (i, v) in a.iter().enumerate() {
					self.insert_value(&format!("{}_{}", key, i), v)?;
				}
			}
			Value::Object(m) => {
				for (k, v) in m.iter() {
					self.insert_value(&format!("{}_{}", key, k.to_ascii_uppercase()), v)?;
				}
			}
		}
		Ok(())
	}

	/// Return a vector of CString NAME=VALUE, consuming self in the process
	pub fn get_envp(self) -> Result<Vec<CString>> {
		let mut res = Vec::with_capacity(self.len());
		for (k, v) in self.into_iter() {
			res.push(CString::new(k + "=" + &v)?);
		}
		Ok(res)
	}
}

impl Deref for Vars {
	type Target = HashMap<String, String>;

	/// Forwards methods to HashMap
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

impl IntoIterator for Vars {
	type Item = (String, String);
	type IntoIter = hash_map::IntoIter<String, String>;

	/// Forwards into_iter to vars
	fn into_iter(self) -> Self::IntoIter {
		self.vars.into_iter()
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use serde_json::json;

	#[test]
	/// test generating a list of variables from a json tree
	fn insert_tree() {
		let value = json!({ "dict": {"key1": "val1", "key2": "val2", "key3": [1, 2, 3, 4]} });
		let mut vars = Vars::default();
		let _ = vars.insert_value("VAR", &value).unwrap();
		let expected: HashMap<&str, &str> = [
			("VAR_DICT_KEY1", "val1"),
			("VAR_DICT_KEY2", "val2"),
			("VAR_DICT_KEY3_0", "1"),
			("VAR_DICT_KEY3_1", "2"),
			("VAR_DICT_KEY3_2", "3"),
			("VAR_DICT_KEY3_3", "4"),
		]
		.iter()
		.cloned()
		.collect();
		vars.into_iter().all(|(k, v)| {
			if expected.contains_key(k.as_str()) {
				assert_eq!(*expected.get(k.as_str()).unwrap(), v);
				true
			} else {
				panic!("missing key {}", k)
			}
		});
	}

	#[test]
	/// test generating a list of variables from a json subtree
	fn insert_subtree() {
		let value = json!(
			{ "data": {"key1": "val1", "key2": "val2", "key3": [1, 2, 3, 4]}, "metadata": {"key4": "val4"} }
		);
		let value = value.pointer("/data").unwrap();
		let mut vars = Vars::default();
		let _ = vars.insert_value("VAR", &value).unwrap();
		let expected: HashMap<&str, &str> = [
			("VAR_KEY1", "val1"),
			("VAR_KEY2", "val2"),
			("VAR_KEY3_0", "1"),
			("VAR_KEY3_1", "2"),
			("VAR_KEY3_2", "3"),
			("VAR_KEY3_3", "4"),
		]
		.iter()
		.cloned()
		.collect();
		vars.into_iter().all(|(k, v)| {
			if expected.contains_key(k.as_str()) {
				assert_eq!(*expected.get(k.as_str()).unwrap(), v);
				true
			} else {
				panic!("missing key {}", k)
			}
		});
	}

	#[test]
	//// test generating a variable from a leaf directly
	fn insert_leaf() {
		let value = json!({ "data": "val1", "metadata": {"key4": "val4"} });
		let value = value.pointer("/data").unwrap();
		let mut vars = Vars::default();
		let _ = vars.insert_value("VAR", &value).unwrap();
		let expected: HashMap<&str, &str> = [("VAR", "val1")].iter().cloned().collect();
		vars.into_iter().all(|(k, v)| {
			if expected.contains_key(k.as_str()) {
				assert_eq!(*expected.get(k.as_str()).unwrap(), v);
				true
			} else {
				panic!("missing key {}", k)
			}
		});
	}
}
