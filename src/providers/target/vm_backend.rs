use anyhow::{anyhow, Context, Result};
use std::env;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

/// Transport used to run privileged shell commands inside the target
/// container runtime's Linux environment.
pub enum VmBackend {
    /// macOS: a Lima-managed VM (Rancher Desktop, Colima) via `limactl shell`.
    Lima { instance: String },
    /// macOS fallback for Rancher Desktop's own `rdctl shell`.
    Rdctl,
    /// Windows or WSL: a WSL2 distro reached via `wsl.exe -d <distro>`.
    /// Works both from a native Windows host and from inside another WSL
    /// distro (WSL supports launching sibling distros via `wsl.exe`).
    WslDistro { distro: String },
    /// Already running inside the target environment, e.g. tbridge executing
    /// directly in the WSL distro that hosts the runtime.
    Local,
}

/// True when the current process is running inside a WSL distro.
pub fn is_wsl() -> bool {
    if env::var_os("WSL_DISTRO_NAME").is_some() || env::var_os("WSL_INTEROP").is_some() {
        return true;
    }

    std::fs::read_to_string("/proc/version")
        .map(|version| version.to_ascii_lowercase().contains("microsoft"))
        .unwrap_or(false)
}

/// Name of the WSL distro currently running this process, if any.
pub fn current_wsl_distro() -> Option<String> {
    env::var("WSL_DISTRO_NAME").ok()
}

/// Lists installed WSL distro names by shelling out to `wsl.exe -l -q`.
/// Works from a native Windows host and, via interop, from inside WSL.
pub fn list_wsl_distros() -> Result<Vec<String>> {
    let output = direct_wsl_command()
        .args(["-l", "-q"])
        .output()
        .context("failed to execute wsl.exe -l -q")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!("wsl.exe -l -q failed: {stderr}"));
    }

    let text = decode_windows_cli_output(&output.stdout);

    Ok(text
        .lines()
        .map(|line| line.trim().trim_end_matches(" (Default)").trim())
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect())
}

fn decode_windows_cli_output(bytes: &[u8]) -> String {
    if bytes.starts_with(&[0xFF, 0xFE]) || looks_like_utf16le(bytes) {
        let start = if bytes.starts_with(&[0xFF, 0xFE]) {
            2
        } else {
            0
        };

        let utf16: Vec<u16> = bytes[start..]
            .chunks_exact(2)
            .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
            .collect();
        return String::from_utf16_lossy(&utf16);
    }

    String::from_utf8_lossy(bytes).into_owned()
}

fn looks_like_utf16le(bytes: &[u8]) -> bool {
    if bytes.len() < 4 || bytes.len() % 2 != 0 {
        return false;
    }

    let sample_pairs = bytes.chunks_exact(2).take(64);
    let mut total = 0usize;
    let mut zero_high_bytes = 0usize;

    for pair in sample_pairs {
        total += 1;
        if pair[1] == 0 {
            zero_high_bytes += 1;
        }
    }

    total >= 4 && zero_high_bytes * 2 >= total
}

impl VmBackend {
    fn label(&self) -> &'static str {
        match self {
            VmBackend::Lima { .. } => "limactl shell",
            VmBackend::Rdctl => "rdctl shell",
            VmBackend::WslDistro { .. } => "wsl.exe",
            VmBackend::Local => "local shell",
        }
    }

    /// Wraps a script so it always runs with root privileges, regardless of
    /// how the backend reaches the target environment.
    fn root_script(&self, script: &str) -> String {
        match self {
            // wsl.exe is invoked with `-u root`, so no elevation needed here.
            VmBackend::WslDistro { .. } => script.to_string(),
            VmBackend::Local => format!(
                "if [ \"$(id -u)\" = 0 ]; then {script}; else sudo sh -c {quoted}; fi",
                quoted = shell_quote(script)
            ),
            VmBackend::Lima { .. } | VmBackend::Rdctl => {
                format!("sudo sh -c {}", shell_quote(script))
            }
        }
    }

    fn build(&self, script: &str) -> Command {
        let rooted = self.root_script(script);
        match self {
            VmBackend::Lima { instance } => {
                let mut cmd = Command::new("limactl");
                cmd.args(["shell", instance, "sh", "-lc", &rooted]);
                cmd
            }
            VmBackend::Rdctl => {
                let mut cmd = Command::new("rdctl");
                cmd.args(["shell", "sh", "-lc", &rooted]);
                cmd
            }
            VmBackend::WslDistro { distro } => {
                let mut cmd = direct_wsl_command();
                cmd.args(["-d", distro, "-u", "root", "--", "sh", "-lc", &rooted]);
                cmd
            }
            VmBackend::Local => {
                let mut cmd = Command::new("sh");
                cmd.args(["-lc", &rooted]);
                cmd
            }
        }
    }

    pub fn run(&self, script: &str) -> Result<()> {
        let output = self
            .execute(script)
            .with_context(|| format!("failed to execute {}", self.label()))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("{} command failed: {stderr}", self.label()));
        }
        Ok(())
    }

    pub fn capture(&self, script: &str) -> Result<String> {
        let output = self
            .execute(script)
            .with_context(|| format!("failed to execute {}", self.label()))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("{} command failed: {stderr}", self.label()));
        }

        String::from_utf8(output.stdout)
            .with_context(|| format!("invalid UTF-8 from {} output", self.label()))
    }

    pub fn run_with_stdin(&self, script: &str, stdin_content: &str) -> Result<()> {
        let mut child = self
            .spawn(script)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .with_context(|| format!("failed to execute {}", self.label()))?;

        if let Some(mut stdin) = child.stdin.take() {
            stdin
                .write_all(stdin_content.as_bytes())
                .with_context(|| format!("failed writing stdin to {}", self.label()))?;
        }

        let output = child
            .wait_with_output()
            .with_context(|| format!("failed waiting for {}", self.label()))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("{} command failed: {stderr}", self.label()));
        }
        Ok(())
    }
}

impl VmBackend {
    fn execute(&self, script: &str) -> Result<std::process::Output> {
        match self {
            VmBackend::WslDistro { .. } => {
                let primary = self.build(script).output();
                if primary.is_ok() {
                    return primary.map_err(Into::into);
                }

                self.fallback_wsl_via_cmd(script)
            }
            _ => self.build(script).output().map_err(Into::into),
        }
    }

    fn spawn(&self, script: &str) -> Command {
        match self {
            VmBackend::WslDistro { distro } => build_wsl_command(distro, &self.root_script(script)),
            _ => self.build(script),
        }
    }

    fn fallback_wsl_via_cmd(&self, script: &str) -> Result<std::process::Output> {
        let VmBackend::WslDistro { distro } = self else {
            return Err(anyhow!("invalid fallback invocation"));
        };

        let rooted = self.root_script(script);
        let mut cmd = Command::new("cmd.exe");
        cmd.args([
            "/C", "wsl.exe", "-d", distro, "-u", "root", "--", "sh", "-lc", &rooted,
        ]);
        cmd.output()
            .map_err(|error| anyhow!("wsl.exe primary+cmd fallback failed: {error}"))
    }
}

fn direct_wsl_command() -> Command {
    if let Some(path) = wsl_executable_path() {
        return Command::new(path);
    }

    Command::new("wsl.exe")
}

fn build_wsl_command(distro: &str, rooted: &str) -> Command {
    let mut cmd = direct_wsl_command();
    cmd.args(["-d", distro, "-u", "root", "--", "sh", "-lc", rooted]);
    cmd
}

fn wsl_executable_path() -> Option<String> {
    let windir = env::var("WINDIR").ok()?;
    let candidates = [
        format!(r"{windir}\System32\wsl.exe"),
        format!(r"{windir}\Sysnative\wsl.exe"),
    ];

    candidates
        .into_iter()
        .find(|candidate| Path::new(candidate).exists())
}

fn shell_quote(input: &str) -> String {
    format!("'{}'", input.replace('\'', "'\\''"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shell_quote_escapes_single_quotes() {
        assert_eq!(shell_quote("it's"), "'it'\\''s'");
    }

    #[test]
    fn finds_wsl_path_when_windir_is_present() {
        if let Ok(windir) = env::var("WINDIR") {
            let maybe = wsl_executable_path();
            if Path::new(&format!(r"{windir}\System32\wsl.exe")).exists()
                || Path::new(&format!(r"{windir}\Sysnative\wsl.exe")).exists()
            {
                assert!(maybe.is_some());
            }
        }
    }

    #[test]
    fn decodes_utf16le_without_bom_for_wsl_list() {
        let bytes = [b'U', 0, b'b', 0, b'u', 0, b'n', 0, b't', 0, b'u', 0];
        let decoded = decode_windows_cli_output(&bytes);
        assert_eq!(decoded, "Ubuntu");
    }
}
