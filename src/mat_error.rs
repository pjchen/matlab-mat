#[derive(Debug)]
pub enum MatError {
    IOError(std::io::Error),
    ParseError(nom::Err<nom::error::Error<&'static [u8]>>),
    ConversionError,
    InternalError,
    ParamsError(String),
}

impl std::fmt::Display for MatError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            MatError::IOError(_) => write!(f, "An I/O error occurred"),
            MatError::ParseError(_) => write!(f, "An error occurred while parsing the file"),
            MatError::ConversionError => {
                write!(f, "An error occurred while converting number formats")
            }
            MatError::InternalError => write!(f, "An internal error occurred, this is a bug"),
            MatError::ParamsError(_) => write!(f, "Params bug"),
        }
    }
}
impl From<String> for MatError {
    fn from(error: String) -> Self {
        MatError::ParamsError(error)
    }
}
impl From<std::io::Error> for MatError {
    fn from(error: std::io::Error) -> Self {
        MatError::IOError(error)
    }
}

impl std::error::Error for MatError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            MatError::IOError(ref err) => Some(err),
            _ => None,
        }
    }
}
