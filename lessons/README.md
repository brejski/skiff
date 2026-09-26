# Skiff lessons

Each lesson adds one piece of functionality. A lesson file explains what we are
building, the background needed to understand it, the steps, and how to test the
result. Lessons are written before they are implemented, and every lesson ends
with a manual code review.

Legend: ✅ done · 🚧 in progress · ⬜ not started

## Part I — Foundations

| # | Lesson | Status |
|---|---|---|
| 0 | [Setup & skeleton](00-setup-and-skeleton.md) — KVM on WSL2, toolchain, repo, CI | 🚧 |
| 1 | Hypervisor theory — KVM vs VMM responsibilities, Firecracker/Cloud Hypervisor architecture, Skiff design doc | ⬜ |
| 2 | Talking to KVM — hand-written ioctls, API version, capabilities, create a VM | ⬜ |
| 3 | Guest memory & first vCPU — `mmap` guest RAM, run real-mode code that prints via port 0x3f8 | ⬜ |
| 4 | The vCPU run loop & VM exits — PIO/MMIO/HLT/shutdown, device bus | ⬜ |
| 5 | Long mode — page tables, GDT, control registers, 64-bit guest | ⬜ |

## Part II — Booting Linux

| # | Lesson | Status |
|---|---|---|
| 6 | Loading a Linux kernel — ELF/bzImage parsing, boot protocol, `boot_params`, E820 | ⬜ |
| 7 | Serial console — 16550 UART emulation, output then input | ⬜ |
| 8 | Interrupts & timers — irqchip/PIT, CPUID, MSRs, MP tables | ⬜ |
| 9 | Minimal guest & initramfs — tiny kernel config, BusyBox, a shell prompt | ⬜ |
| 10 | Multiple vCPUs — thread per vCPU, shutdown/reboot | ⬜ |

## Part III — Virtio

| # | Lesson | Status |
|---|---|---|
| 11 | Virtio fundamentals — virtio-mmio, feature negotiation, split virtqueues | ⬜ |
| 12 | Event-driven I/O — eventfd, ioeventfd/irqfd, epoll loop | ⬜ |
| 13 | virtio-block — file-backed disk, ext4 root filesystem | ⬜ |
| 14 | virtio-net — TAP backend, host networking | ⬜ |
| 15 | virtio-rng & virtio-vsock (optional) | ⬜ |

## Part IV — Making it a real VMM

| # | Lesson | Status |
|---|---|---|
| 16 | Configuration & CLI | ⬜ |
| 17 | API server — REST over a Unix socket | ⬜ |
| 18 | Jailer & seccomp | ⬜ |
| 19 | Boot time & performance | ⬜ |

## Part V — GPU (stretch)

| # | Lesson | Status |
|---|---|---|
| 20 | Passthrough theory — PCI, IOMMU, VFIO | ⬜ |
| 21 | PCI bus — config space, BARs, MSI-X | ⬜ |
| 22 | GPU access — VFIO passthrough or virtio-gpu | ⬜ |

## Part VI — Moving to rust-vmm

| # | Lesson | Status |
|---|---|---|
| 23 | Survey — our code vs the rust-vmm crates | ⬜ |
| 24 | KVM & memory layer — `kvm-ioctls`, `vm-memory` | ⬜ |
| 25 | Boot & devices — `linux-loader`, `vm-superio`, `virtio-queue` | ⬜ |
| 26 | Event loop & hardening — `event-manager`, seccomp, fuzzing | ⬜ |

## Part VII — Other platforms

| # | Lesson | Status |
|---|---|---|
| 27 | x86 vs ARM64 theory | ⬜ |
| 28 | Arch abstraction refactor | ⬜ |
| 29 | ARM64 vCPU on KVM | ⬜ |
| 30 | Hand-written FDT generator, boot arm64 Linux | ⬜ |
| 31 | ARM devices — vGICv3, timer, PSCI, PL011 | ⬜ |
| 32 | Hypervisor.framework vs KVM theory | ⬜ |
| 33 | Apple Silicon backend | ⬜ |
