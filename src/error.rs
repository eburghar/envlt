#[derive(Debug, thiserror::Error)]
pub enum Error {
	#[error("missing a \":\" to separate backend from arguments \"{0}\"")]
	NoArgs(String),
	#[error("missing a \":\" to separate arguments from path \"{0}\"")]
	NoPath(String),
	#[error("missing a \":\" to separate path from json pointer \"{0}\"")]
	NoSubpath(String),
	#[error("extra data after json pointer \"{0}\"")]
	ExtraData(String),
	#[error(transparent)]
	NulError(#[from] std::ffi::NulError),
	#[error("json pointer \"{0}\" returns no result")]
	Pointer(String),
	#[error("variable expression should be prefix=path: \"{0}\"")]
	ParseVar(String),
	#[error(transparent)]
	VaultError(#[from] vault_jwt::error::Error),
	#[error("getting token {0}")]
	TokenError(#[from] std::io::Error),
    #[error("executing {0:?}")]
    ExecError(std::ffi::CString, #[source] std::io::Error),
    #[error("missing role in {0}")]
    MissingRole(String)
}

pub type Result<T> = std::result::Result<T, Error>;
