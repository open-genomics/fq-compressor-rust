// =============================================================================
// fqc-rust - Verify Command
// =============================================================================

use crate::println_stdout;
pub use crate::verify::{verify_archive, VerifyOptions, VerifyReport};

// =============================================================================
// VerifyCommand
// =============================================================================

pub struct VerifyCommand {
    opts: VerifyOptions,
}

impl VerifyCommand {
    pub fn new(opts: VerifyOptions) -> Self {
        Self { opts }
    }

    pub fn execute(self) -> i32 {
        match verify_archive(&self.opts) {
            Ok(report) => {
                if report.passed {
                    println_stdout!(
                        "Verification PASSED: {} blocks checked, {} reads verified",
                        report.blocks_checked,
                        report.reads_verified
                    );
                    0
                } else {
                    eprintln!(
                        "Verification FAILED: {}/{} blocks had errors",
                        report.blocks_failed, report.blocks_checked
                    );
                    1
                }
            }
            Err(e) => {
                eprintln!("Verify failed: {e}");
                e.exit_code_num()
            }
        }
    }
}
