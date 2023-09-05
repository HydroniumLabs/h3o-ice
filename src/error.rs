use std::{error::Error, fmt};

/// Errors occurring while building a set or a map.
#[derive(Debug)]
#[non_exhaustive]
pub enum BuildError {
    /// Failed to build the underlying FST.
    Fst(fst::Error),
}

impl fmt::Display for BuildError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::Fst(ref err) => write!(f, "FST error: {err}"),
        }
    }
}

impl Error for BuildError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match *self {
            Self::Fst(ref err) => Some(err),
        }
    }
}

impl From<fst::Error> for BuildError {
    fn from(err: fst::Error) -> Self {
        Self::Fst(err)
    }
}
