# Lesson 0 — Setup & Skeleton

## Goal

By the end of this lesson we have:

- a working KVM device (`/dev/kvm`) inside WSL2 that our user can open,
- a Rust toolchain,
- a public GitHub repository `skiff` with a Cargo workspace skeleton,
- CI that checks formatting, lints and tests on every push,
- a `skiff` binary that does exactly one thing: checks whether KVM is usable and explains what's wrong if it isn't.

No virtualization happens yet. This lesson is about having a solid place to build from.

## Background

### Why `/dev/kvm` matters

KVM is the part of the Linux kernel that uses the CPU's virtualization extensions (Intel VT-x, `vmx` in `/proc/cpuinfo`). It exposes itself to userspace as a character device, `/dev/kvm`. Everything our hypervisor does — creating a VM, adding memory, running vCPUs — goes through `ioctl` calls on file descriptors that start from opening this device. No `/dev/kvm`, no hypervisor.

### KVM inside WSL2

WSL2 is itself a VM running on Hyper-V. To run *our* VMs inside it we need **nested virtualization**: Hyper-V has to expose VT-x to the WSL2 guest. On Windows 11 this is enabled by default; it can be forced with `.wslconfig`.

The WSL2 kernel ships KVM as loadable modules (`CONFIG_KVM_INTEL=m`), so we also have to load `kvm_intel` and make sure our user has permission to open the device.

### Cargo workspace

A workspace is a set of crates that share one `Cargo.lock` and one `target/` directory. We start with a single binary crate, but the workspace gives us room to split things later (e.g. a separate crate for guest test programs, or when we compare with rust-vmm in Part VI) without restructuring the repo.

## Steps

### 1. Enable KVM in WSL2 (student, manual)

1. Check the CPU supports virtualization (already confirmed: `vmx` is present):
   ```sh
   grep -m1 -oE 'vmx|svm' /proc/cpuinfo
   ```
2. On the **Windows** side, make sure nested virtualization is enabled. Create or edit `C:\Users\<you>\.wslconfig`:
   ```ini
   [wsl2]
   nestedVirtualization=true
   ```
   Then in PowerShell: `wsl --shutdown`, and reopen the WSL terminal.
3. Load the module and check the device:
   ```sh
   sudo modprobe kvm_intel
   ls -l /dev/kvm
   ```
   If `modprobe` fails with "Operation not supported", nested virtualization isn't active — revisit step 2.
4. Give your user access without `sudo`. WSL doesn't run udev, so the device comes up as
   `root:root` with mode `600` and group membership alone is not enough — the group and mode
   have to be set too:
   ```sh
   sudo usermod -aG kvm $USER          # once; needs a WSL restart to take effect
   sudo chgrp kvm /dev/kvm             # after every WSL start, unless automated in step 5
   sudo chmod 660 /dev/kvm
   ```
   Check with `id` that `kvm` is listed, and with `ls -l /dev/kvm` that the group is `kvm`.
5. Make it stick across WSL restarts by doing both the load and the permissions at boot.
   Add to `/etc/wsl.conf`:
   ```ini
   [boot]
   command = "/bin/sh -c 'modprobe kvm_intel && chgrp kvm /dev/kvm && chmod 660 /dev/kvm'"
   ```
   Then `wsl --shutdown` and reopen. (`/etc/modules-load.d/kvm.conf` loads the module when
   systemd is enabled, but it does not fix the ownership, so the `[boot]` command is the
   one-stop option.)

### 2. Install Rust (student, manual)

```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"
rustup component add rustfmt clippy
cargo --version
```

### 3. Git identity check

`git config --global user.email` should be the email linked to your GitHub account, otherwise commits won't be attributed to you.

### 4. Repository layout (implementation)

```
skiff/
├── Cargo.toml              # [workspace] root, shared lints and profile settings
├── Cargo.lock
├── rust-toolchain.toml     # pin the stable channel + rustfmt/clippy
├── .gitignore              # /target
├── LICENSE                 # Apache-2.0 (same as Firecracker/Cloud Hypervisor, so we can borrow)
├── README.md               # what Skiff is, how to build, link to lessons
├── CLAUDE.md
├── .github/workflows/ci.yml
├── lessons/
│   ├── README.md           # lesson index (outline from CLAUDE.md)
│   └── 00-setup-and-skeleton.md
└── skiff/               # the VMM binary crate
    ├── Cargo.toml
    └── src/
        └── main.rs
```

Later lessons will add modules under `skiff/src/` (`kvm/`, `memory/`, `arch/x86_64/`, `devices/`, `virtio/`) — we don't create empty placeholders now.

### 5. The `skiff` binary

Zero dependencies — just the standard library. `main.rs`:

- opens `/dev/kvm` for reading and writing with `std::fs::OpenOptions`,
- on success prints `KVM is available` and exits with code 0,
- on failure matches on `std::io::ErrorKind` and prints a helpful hint:
  - `NotFound` → module not loaded / nested virtualization disabled,
  - `PermissionDenied` → user not in the `kvm` group,
  - anything else → print the raw error,
- and exits with a non-zero code.

The check lives in a small function returning `Result<(), KvmCheckError>` so `main` only handles printing and the exit code. A unit test covers the error-to-hint mapping (we can't rely on `/dev/kvm` existing in tests).

Rust concepts in this lesson: crates and workspaces, `Result` and `?`, `std::io::Error` and `ErrorKind`, `match`, `impl Display` for our own error type, `std::process::ExitCode`, `#[cfg(test)]` unit tests.

### 6. Continuous integration

`.github/workflows/ci.yml` runs on push and pull requests, on `ubuntu-latest`:

1. `cargo fmt --all -- --check`
2. `cargo clippy --all-targets -- -D warnings`
3. `cargo test`
4. `cargo build --release`

Tests that need a real `/dev/kvm` will come in Lesson 2. They'll be marked `#[ignore]` and run locally with `cargo test -- --ignored`. (GitHub's Linux runners do expose KVM, so we may enable them in CI then.)

### 7. Publish to GitHub

1. `git init -b main`, commit the skeleton.
2. `gh repo create brejski/skiff --public --source . --push`
3. Check the CI run passes: `gh run watch`.

## How to test

| Check | Command | Expected |
|---|---|---|
| KVM device exists | `ls -l /dev/kvm` | `crw-rw---- 1 root kvm ...` |
| User can use it | `id` | groups include `kvm` |
| Toolchain | `cargo --version`, `cargo clippy --version` | versions print |
| Builds & tests | `cargo test` | all tests pass |
| Binary works | `cargo run` | `KVM is available` |
| Error path | `sudo rmmod kvm_intel && cargo run` | hint about loading the module, non-zero exit (`sudo modprobe kvm_intel` afterwards) |
| CI | GitHub → Actions | green run on `main` |

## Student review checklist

- [ ] Read `Cargo.toml` (workspace) and `skiff/Cargo.toml` — understand what each section does.
- [ ] Read `main.rs` — follow how the `io::Error` becomes a hint, and why `main` returns `ExitCode`.
- [ ] Read `ci.yml` — know what each step checks.
- [ ] Break something on purpose (bad formatting, an unused variable) and watch CI fail.
- [ ] Explain in one sentence why `/dev/kvm` is the entry point to everything we'll build.
