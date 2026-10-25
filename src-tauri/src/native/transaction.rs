//! Keep rollback semantics identical for CLI and desktop credential writes.
use crate::error::{Error, Result};
pub fn apply(
    write_and_verify: impl FnOnce() -> Result<()>,
    restore: impl FnOnce() -> Result<()>,
) -> Result<()> {
    match write_and_verify() {
        Ok(()) => Ok(()),
        Err(error) => {
            if restore().is_err() {
                Err(Error::RollbackFailed)
            } else {
                Err(error)
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    #[test]
    fn failed_verification_restores_partial_write() {
        let live = Cell::new(1);
        let result = apply(
            || {
                live.set(2);
                Err(Error::VerifyFailed)
            },
            || {
                live.set(1);
                Ok(())
            },
        );
        assert!(matches!(result, Err(Error::VerifyFailed)));
        assert_eq!(live.get(), 1);
    }
    #[test]
    fn failed_restore_is_never_reported_as_normal_failure() {
        assert!(matches!(
            apply(|| Err(Error::VerifyFailed), || Err(Error::AccountNotFound)),
            Err(Error::RollbackFailed)
        ));
    }
    #[test]
    fn successful_commit_does_not_restore() {
        apply(|| Ok(()), || panic!("must not restore a successful switch")).unwrap();
    }
}
