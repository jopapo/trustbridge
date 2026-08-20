use anyhow::{anyhow, Result};
use std::env;
use std::process::{Command, Stdio};

#[derive(Clone)]
pub struct ContainerRuntime {
    command: String,
}

impl ContainerRuntime {
    pub fn detect() -> Result<Self> {
        if let Some(command) = configured_command() {
            return Ok(Self { command });
        }

        for candidate in ["docker", "nerdctl"] {
            if command_exists(candidate) {
                return Ok(Self {
                    command: candidate.to_string(),
                });
            }
        }

        Err(anyhow!(
            "no compatible container CLI found; install docker or nerdctl, or set TBRIDGE_CONTAINER_CLI"
        ))
    }

    pub fn command(&self) -> &str {
        &self.command
    }

    pub fn name(&self) -> &str {
        &self.command
    }
}

fn configured_command() -> Option<String> {
    env::var("TBRIDGE_CONTAINER_CLI")
        .or_else(|_| env::var("TBRIDGE_CONTAINER_RUNTIME"))
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn command_exists(command: &str) -> bool {
    Command::new(command)
        .arg("--help")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok()
}
