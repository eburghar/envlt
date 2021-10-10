#[derive(Debug, thiserror::Error)]
pub enum Error {
	#[error(transparent)]
	NulError(#[from] std::ffi::NulError),
	#[error(transparent)]
	VaultError(#[from] vault_jwt::error::Error),
	#[error("Executing {0:?}")]
	ExecError(std::ffi::CString, #[source] std::io::Error),
	#[error("Missing role in {0}")]
	MissingRole(String),
	#[error("Parsing {0}")]
	ParseError(String, #[source] serde_json::error::Error),
    #[error("Expected argument {0} on {1}")]
    ExpectedArg(String, String),
	#[error("json pointer \"{0}\" returns no result")]
	Pointer(String),
}

pub type Result<T> = std::result::Result<T, Error>;
