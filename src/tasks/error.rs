use std::fmt::{self, Display};

/// Can be used in functions with return type `Result<(), TaskError>`.
///
/// ```ignore
/// task_panic!("Message");
/// ```
macro_rules! task_panic {
    ($message:expr) => {
        return Err(TaskError::from_message($message.to_string()));
    };
}
pub(crate) use task_panic;

/// Can be used in functions with return type `Result<(), TaskError>`.
/// The `error` can be anything that implements the `std::error::Error` trait.
///
/// ```ignore
/// task_error_panic!("message", error);
/// ```
macro_rules! task_error_panic {
    ($message:expr, $error:expr) => {
        return Err(TaskError::from_error(
            $message.to_string(),
            SourceError::DynError(Box::new($error)),
        ))
    };
}
pub(crate) use task_error_panic;

/// Will take a piece of code and a message.
/// If the executed code returns Ok(value) the macro returns the value.
/// If Err gets returned `task_error_panic` gets called with the message and the error.
///
/// Can be used in functions with return type `Result<(), TaskError>`.
///
/// ```ignore
/// task_might_panic!(code_that_would_need_to_be_unwrapped, "Message");
/// ```
macro_rules! task_might_panic {
    ($code:expr, $message:expr) => {
        match $code {
            Ok(value) => value,
            Err(error) => task_error_panic!($message, error),
        }
    };
}
pub(crate) use task_might_panic;

#[derive(Debug)]
pub enum SourceError {
    TaskError(Box<TaskError>),
    DynError(Box<dyn std::error::Error>),
    None,
}

#[derive(Debug)]
pub struct TaskError {
    message: String,
    source_error: SourceError,
}

impl TaskError {
    pub fn new(message: String, source_error: SourceError) -> Self {
        Self {
            message,
            source_error,
        }
    }

    pub fn from_error(message: String, source_error: SourceError) -> Self {
        Self {
            message,
            source_error,
        }
    }

    pub fn from_message(message: String) -> Self {
        Self {
            message,
            source_error: SourceError::None,
        }
    }
}

impl Display for TaskError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "TaskError {}. Source: {:?}",
            self.message, self.source_error
        )
    }
}

impl From<Box<dyn std::error::Error>> for TaskError {
    fn from(value: Box<dyn std::error::Error>) -> Self {
        Self {
            message: value.to_string(),
            source_error: SourceError::DynError(value),
        }
    }
}

impl std::error::Error for TaskError {}
