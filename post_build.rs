use std::env;
use std::fmt;
use std::fs;
use std::path::Path;

#[derive(Debug)]
enum PostError {
    VarError(env::VarError),
    PathError(std::io::Error),
}

impl std::error::Error for PostError {}

impl fmt::Display for PostError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            PostError::VarError(ref err) => err.fmt(f),
            PostError::PathError(ref err) => err.fmt(f),
        }
    }
}

fn main() -> std::result::Result<(), PostError> {
    let crate_path = env::var("CRATE_OUT_DIR")?;
    let manifest_path = env::var("CRATE_MANIFEST_DIR")?;

    fs::create_dir_all(Path::new(&manifest_path).join("bazel-bin"))?;
    fs::copy(
        Path::new(&crate_path).join("token_authn.wasm"),
        Path::new(&manifest_path).join("bazel-bin/token_authn.wasm"),
    )?;

    Ok(())
}

impl From<env::VarError> for PostError {
    fn from(err: env::VarError) -> Self {
        PostError::VarError(err)
    }
}

impl From<std::io::Error> for PostError {
    fn from(err: std::io::Error) -> Self {
        PostError::PathError(err)
    }
}
