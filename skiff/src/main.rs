//! Skiff — a small KVM-based hypervisor for micro VMs.
//!
//! Lesson 0 does not create a virtual machine yet. It answers the one question
//! every later lesson depends on: can we talk to KVM at all?
//!
//! KVM is the part of the Linux kernel that drives the CPU's virtualization
//! extensions (Intel VT-x / AMD-V). Userspace reaches it through a character
//! device, `/dev/kvm`. Creating a VM, giving it memory and running a vCPU are
//! all `ioctl` calls on file descriptors that descend from opening this file,
//! so if we cannot open it, there is nothing else to try.

use std::fmt;
use std::fs::{File, OpenOptions};
use std::io::{self, ErrorKind};
use std::process::ExitCode;

/// The KVM character device.
const KVM_DEVICE: &str = "/dev/kvm";

/// We could not open the KVM device.
///
/// We keep the original [`io::Error`] instead of flattening it to a string:
/// the caller may want the raw errno, and the `ErrorKind` is what lets us give
/// a useful hint.
#[derive(Debug)]
struct KvmUnavailable {
    path: &'static str,
    source: io::Error,
}

impl KvmUnavailable {
    /// A human-sized suggestion for the most likely cause.
    ///
    /// The three interesting cases on a WSL2 dev box are: the module is not
    /// loaded (`NotFound`), the device exists but our user cannot open it
    /// (`PermissionDenied`), and everything else.
    fn hint(&self) -> &'static str {
        match self.source.kind() {
            ErrorKind::NotFound => {
                "the KVM module does not seem to be loaded. Try `sudo modprobe kvm_intel`. \
                 If that fails, nested virtualization is probably off: set \
                 `nestedVirtualization=true` under `[wsl2]` in C:\\Users\\<you>\\.wslconfig, \
                 then run `wsl --shutdown` in PowerShell."
            }
            ErrorKind::PermissionDenied => {
                "the device exists but this user cannot open it. Make it group-readable \
                 (`sudo chgrp kvm /dev/kvm && sudo chmod 660 /dev/kvm`) and join that group \
                 (`sudo usermod -aG kvm $USER`), then restart WSL so the membership applies."
            }
            _ => "this error is not one we have a specific hint for yet.",
        }
    }
}

impl fmt::Display for KvmUnavailable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "cannot open {}: {}", self.path, self.source)
    }
}

impl std::error::Error for KvmUnavailable {
    /// Lets callers walk the error chain down to the underlying I/O error.
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.source)
    }
}

/// Open the KVM device read-write.
///
/// Read-write because later lessons issue `ioctl`s that mutate kernel state and
/// `mmap` the shared `kvm_run` page; `O_RDONLY` would not be enough.
///
/// Returning the open [`File`] rather than `()` means the caller owns the file
/// descriptor: Rust closes it when the value is dropped, so there is no
/// `close()` for us to forget.
fn open_kvm() -> Result<File, KvmUnavailable> {
    OpenOptions::new()
        .read(true)
        .write(true)
        .open(KVM_DEVICE)
        .map_err(|source| KvmUnavailable {
            path: KVM_DEVICE,
            source,
        })
}

/// `main` only reports: the work lives in [`open_kvm`], the exit status is
/// expressed with [`ExitCode`] so the shell can tell success from failure.
fn main() -> ExitCode {
    match open_kvm() {
        Ok(kvm) => {
            println!("KVM is available at {KVM_DEVICE}");
            // Dropping the File closes the descriptor. Explicit here only to
            // show where it happens; it would happen at end of scope anyway.
            drop(kvm);
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("error: {err}");
            eprintln!("hint: {}", err.hint());
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build an error as if opening the device had failed with `kind`.
    /// Tests must not depend on whether this machine actually has KVM.
    fn unavailable(kind: ErrorKind) -> KvmUnavailable {
        KvmUnavailable {
            path: KVM_DEVICE,
            source: io::Error::from(kind),
        }
    }

    #[test]
    fn missing_device_suggests_loading_the_module() {
        let hint = unavailable(ErrorKind::NotFound).hint();
        assert!(hint.contains("modprobe kvm_intel"), "got: {hint}");
    }

    #[test]
    fn permission_denied_suggests_the_kvm_group() {
        let hint = unavailable(ErrorKind::PermissionDenied).hint();
        assert!(hint.contains("usermod -aG kvm"), "got: {hint}");
    }

    #[test]
    fn unknown_errors_still_get_a_hint() {
        assert!(!unavailable(ErrorKind::Other).hint().is_empty());
    }

    #[test]
    fn display_mentions_the_device_path() {
        let shown = unavailable(ErrorKind::NotFound).to_string();
        assert!(shown.contains(KVM_DEVICE), "got: {shown}");
    }
}
