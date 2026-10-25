//! Keychain access through `/usr/bin/security`, matching `go-keyring`.
//!
//! Calling the system tool binds each item's ACL to `/usr/bin/security`,
//! avoiding keychain prompts across OrbitSwitch updates and Antigravity runs.
//! Writes pass through `security -i` on stdin so secrets stay off the process list.

use std::io::Write;
use std::process::{Command, Stdio};

use crate::error::{Error, Result};

const SECURITY: &str = "/usr/bin/security";
/// `security -i` rejects interactive lines longer than this.
const MAX_COMMAND_LEN: usize = 4096;
/// `errSecItemNotFound` as reported in the exit status.
const NOT_FOUND_STATUS: i32 = 44;

pub fn get(service: &str, account: &str) -> Result<Option<String>> {
    let output = Command::new(SECURITY)
        .args(["find-generic-password", "-s", service, "-a", account, "-w"])
        .stdin(Stdio::null())
        .output()?;
    match output.status.code() {
        Some(0) => {
            let value = String::from_utf8(output.stdout)
                .map_err(|_| Error::SecretStore("keychain item is not UTF-8".into()))?;
            Ok(Some(value.trim_end_matches('\n').to_owned()))
        }
        Some(NOT_FOUND_STATUS) => Ok(None),
        status => Err(Error::SecretStore(format!(
            "find-generic-password exited with {status:?}"
        ))),
    }
}

pub fn set(service: &str, account: &str, secret: &str) -> Result<()> {
    let line = format!(
        "add-generic-password -U -s {} -a {} -w {}\n",
        quote(service),
        quote(account),
        quote(secret)
    );
    if line.len() > MAX_COMMAND_LEN {
        return Err(Error::SecretStore(
            "secret exceeds the keychain line limit".into(),
        ));
    }
    let mut child = Command::new(SECURITY)
        .arg("-i")
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()?;
    child
        .stdin
        .take()
        .ok_or_else(|| Error::SecretStore("security stdin unavailable".into()))?
        .write_all(line.as_bytes())?;
    let output = child.wait_with_output()?;
    // `security -i` exits 0 even when a command fails, so stderr is the signal.
    if !output.status.success() || !output.stderr.is_empty() {
        return Err(Error::SecretStore(format!(
            "add-generic-password failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    Ok(())
}

pub fn delete(service: &str, account: &str) -> Result<()> {
    let status = Command::new(SECURITY)
        .args(["delete-generic-password", "-s", service, "-a", account])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()?;
    match status.code() {
        Some(0) | Some(NOT_FOUND_STATUS) => Ok(()),
        code => Err(Error::SecretStore(format!(
            "delete-generic-password exited with {code:?}"
        ))),
    }
}

/// Single-quote shell escaping, as understood by `security -i`.
fn quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', r"'\''"))
}

#[cfg(test)]
mod tests {
    use super::quote;

    #[test]
    fn quotes_plain_and_single_quote_values() {
        assert_eq!(quote("gemini"), "'gemini'");
        assert_eq!(quote("it's"), r"'it'\''s'");
    }
}
