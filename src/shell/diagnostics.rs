use super::{LoadFailure, RuntimeBootstrapError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellDiagnostic {
    pub title: String,
    pub message: String,
    pub detail: Option<String>,
}

impl ShellDiagnostic {
    pub fn from_load_failure(failure: &LoadFailure) -> Self {
        Self {
            title: String::from("ROM Load Failed"),
            message: failure.message.clone(),
            detail: Some(format!(
                "Attempted path: {}",
                failure.attempted_path.display()
            )),
        }
    }

    pub fn from_runtime_bootstrap_error(error: &RuntimeBootstrapError) -> Self {
        Self {
            title: String::from("Runtime View Failed to Start"),
            message: error.diagnostic_message(),
            detail: None,
        }
    }

    pub fn render(&self) -> String {
        match &self.detail {
            Some(detail) => format!("{}\n{}\n{}", self.title, self.message, detail),
            None => format!("{}\n{}", self.title, self.message),
        }
    }
}
