use std::borrow::Cow;
use std::ffi::CStr;
use std::format;
use std::string::String;

use crate::abort::abort_with_id;

pub struct Error {
    error_id: Cow<'static, str>,
    message: String,
}

impl Error {
    pub fn new(error_id: impl Into<Cow<'static, str>>, message: impl Into<String>) -> Self {
        Self {
            error_id: error_id.into(),
            message: message.into(),
        }
    }

    pub fn throw(self) -> ! {
        let id = format!("{}:{}\0", ERROR_ID_COMPONENT, self.error_id);
        let id = unsafe { CStr::from_bytes_with_nul_unchecked(id.as_bytes()) };
        abort_with_id(id, self.message)
    }
}

static ERROR_ID_COMPONENT: &str = env!("ERROR_ID_COMPONENT");

impl<E> From<E> for Error
where
    E: std::error::Error + 'static,
{
    fn from(err: E) -> Self {
        Error::new("GenericError", format!("{:?}", err))
    }
}
