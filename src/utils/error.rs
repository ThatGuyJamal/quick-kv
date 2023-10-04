use std::fmt;

pub struct QuickKVError(String);

impl fmt::Display for QuickKVError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "QuickKvError: {}", self.0)
    }
}

impl fmt::Debug for QuickKVError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "QuickKvError: {}", self.0)
    }
}

impl std::error::Error for QuickKVError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        None
    }

    fn description(&self) -> &str {
        &self.0
    }

    fn cause(&self) -> Option<&dyn std::error::Error> {
        None
    }
}
