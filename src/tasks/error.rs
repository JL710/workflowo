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
/// task_dynerror_panic!("message", error);
/// ```
macro_rules! task_dynerror_panic {
    ($message:expr, $error:expr) => {
        return Err(TaskError::from_error(
            $message.to_string(),
            SourceError::DynError(Box::new($error)),
        ))
    };
}
pub(crate) use task_dynerror_panic;

/// Can be used in functions with return type [Result<(), TaskError>].
/// The `error` must be a [TaskError].
///
/// ```ignore
/// task_taskerror_panic!("message", error);
/// ```
macro_rules! task_taskerror_panic {
    ($message:expr, $error:expr) => {
        return Err(TaskError::from_taskerror($message.to_string(), $error))
    };
}
pub(crate) use task_taskerror_panic;

/// Will take a piece of code and a message.
/// If the executed code returns Ok(value) the macro returns the value.
/// If Err gets returned `task_dynerror_panic` gets called with the message and the error.
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
            Err(error) => task_dynerror_panic!($message, error),
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

    pub fn from_taskerror(message: String, source_error: TaskError) -> Self {
        Self {
            message,
            source_error: SourceError::TaskError(Box::new(source_error)),
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
        match &self.source_error {
            SourceError::None => write!(
                f,
                "{}",
                &self
                    .message
                    .lines()
                    .map(|x| "║ ".to_owned() + x + "\n")
                    .collect::<String>()
            ),
            SourceError::TaskError(error) => write!(
                f,
                "{}\n{}",
                self.message
                    .lines()
                    .map(|x| "║ ".to_owned() + x + "\n╠══ Caused by")
                    .collect::<String>(),
                error
            ),
            SourceError::DynError(error) => write!(f, "{}\n{:?}", self.message, error),
        }
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
