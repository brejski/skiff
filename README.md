# Skiff

A small hypervisor (VMM) written in Rust, built to run micro VMs — and built by
hand, one lesson at a time, to learn both Rust and how virtualization actually
works under Linux/KVM.

Skiff borrows ideas from [Firecracker](https://github.com/firecracker-microvm/firecracker)
and [Cloud Hypervisor](https://github.com/cloud-hypervisor/cloud-hypervisor) but
does not try to compete with them. Where they use the excellent
[rust-vmm](https://github.com/rust-vmm) crates, we first write the equivalent
ourselves — ioctl definitions, guest memory, the Linux boot protocol,
virtqueues — because the point is understanding, not production readiness.
A later part of the course swaps our code for rust-vmm and compares the two.

**Status: Lesson 0.** The binary checks that KVM is usable. No VM runs yet.

## Roadmap

Boot a tiny Linux kernel in a micro VM, give it a serial console, then virtio
block and network devices, then a real CLI and API, and finally GPU access.
The full lesson list lives in [`lessons/`](lessons/README.md).

## Requirements

- Linux with KVM (`/dev/kvm`) and an x86_64 CPU with VT-x or AMD-V
- a stable Rust toolchain (see `rust-toolchain.toml`)

On WSL2, KVM needs nested virtualization and the `kvm_intel` module; see
[Lesson 0](lessons/00-setup-and-skeleton.md) for the setup steps.

## Build and run

```sh
cargo build
cargo run          # prints whether KVM is available
cargo test
cargo clippy --all-targets -- -D warnings
```

## Name

A skiff is a small, light boat — the kind one person builds in a shed. It seemed
fitting for a hypervisor small enough to read end to end.

## License

[Apache-2.0](LICENSE), the same license as Firecracker and Cloud Hypervisor, so
that code and ideas can flow between them and this project.
