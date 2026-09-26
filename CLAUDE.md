Skiff is a new hypervisor written in Rust tailored for running micro VMs. The project is likely to borrow a lot from Firecracker and Cloud Hypervisor but it's not aimed to compete with them. This is mainly a learning project, where we are going to build a working hypervisor that can run a tiny Linux kernel in a micro VM. We also want to be able to use virtio devices, we'll start with block device and network device but the ultimate goal is to also be able to access GPU from the host machine.

The idea of this project is to learn both Rust and internals of building a hypervisor.

## How we work

- The project is split into lessons. Each lesson adds one piece of functionality and lives in `lessons/NN-short-name.md`, explaining what we build, why, and how to test it.
- For each lesson: first write the plan (the lesson md), the student reviews it, then we implement it. Every lesson ends with a manual code review by the student.
- Implement one lesson at a time. Don't pull work from later lessons forward.
- Explain Rust concepts as they come up (ownership, `unsafe`, FFI, traits, ...) — the student is learning Rust too.

## Key decisions

- **Hand-written first.** Write hypervisor components ourselves: KVM ioctl numbers and `#[repr(C)]` structs, guest memory, ELF/bzImage loading, boot params, UART, virtqueues, event loop, devices. In Parts I–V allowed dependencies are `libc` plus small general-purpose crates (error handling, logging, CLI parsing). Do **not** use rust-vmm crates (`kvm-ioctls`, `kvm-bindings`, `vm-memory`, `linux-loader`, `vm-superio`, `virtio-queue`, `event-manager`, ...) until Part VI. Production quality is not the goal; understanding is.
- **Swappable boundaries.** Keep clear module boundaries (KVM, guest memory, vCPU, device bus, virtqueue) and write tests for them, so Part VI can replace pieces with rust-vmm using the same tests.
- **x86_64 on Linux KVM first.** Keep x86-specific code in `arch/x86_64` and KVM-specific code in its own module from the start, but don't introduce abstraction traits until the second implementation exists (Part VII).
- **Dev environment** is WSL2 on an Intel CPU. KVM is available as a kernel module (`kvm_intel`) and needs nested virtualization. Real GPU passthrough (VFIO) is unlikely to work on WSL2 and may need bare-metal Linux.

## Lesson outline

### Part I — Foundations
0. Setup & skeleton — KVM on WSL2, Rust toolchain, GitHub repo, Cargo workspace, CI, `lessons/` folder.
1. Hypervisor theory — KVM vs VMM responsibilities, Firecracker / Cloud Hypervisor architecture, design doc for Skiff.
2. Talking to KVM — open `/dev/kvm`, hand-written ioctl definitions, `KVM_GET_API_VERSION`, capabilities, create a VM.
3. Guest memory & first vCPU — `mmap` guest RAM, safe guest-memory wrapper, run hand-assembled real-mode code that prints via port 0x3f8.
4. The vCPU run loop & VM exits — PIO/MMIO/HLT/shutdown handling, device bus abstraction.
5. Long mode — page tables, GDT, control registers; boot the vCPU directly into 64-bit mode.

### Part II — Booting Linux
6. Loading a Linux kernel — own ELF/bzImage parsing, Linux boot protocol, `boot_params`, E820, cmdline.
7. Serial console — own 16550 UART emulation, output then input.
8. Interrupts & timers — in-kernel irqchip/PIT, CPUID, MSRs, MP tables/ACPI.
9. Minimal guest & initramfs — tiny kernel config, BusyBox initramfs, shell prompt.
10. Multiple vCPUs — thread per vCPU, shared state, shutdown/reboot.

### Part III — Virtio
11. Virtio fundamentals — virtio-mmio transport, feature negotiation, own split virtqueue implementation.
12. Event-driven I/O — eventfd, ioeventfd/irqfd, own epoll event loop.
13. virtio-block — file-backed disk, ext4 root filesystem.
14. virtio-net — TAP backend, RX/TX queues, host networking/NAT.
15. virtio-rng & virtio-vsock (optional).

### Part IV — Making it a real VMM
16. Configuration & CLI.
17. API server — REST over a Unix socket (Firecracker-style).
18. Jailer & seccomp — namespaces, chroot, dropping privileges, seccomp filters.
19. Boot time & performance — measure, hugepages, kernel trimming.

### Part V — GPU (stretch)
20. Passthrough theory — PCI, IOMMU, VFIO, what's feasible on WSL2.
21. PCI bus — host bridge, config space, BARs, MSI-X.
22. GPU access — (a) VFIO passthrough on bare metal, or (b) virtio-gpu (virglrenderer/rutabaga/venus).

### Part VI — Moving to rust-vmm
23. Survey — compare our code with rust-vmm crates; what they handle that we didn't.
24. KVM & memory layer — `kvm-ioctls`/`kvm-bindings`, `vm-memory`.
25. Boot & devices — `linux-loader`, `vm-superio`, `virtio-queue`/`virtio-device`.
26. Event loop & hardening — `event-manager`, `vmm-sys-util`, fuzzing virtio paths.

### Part VII — Other platforms
27. x86 vs ARM64 theory — exception levels, GIC vs APIC, MMIO-only, device tree, arm64 `Image` boot, PSCI.
28. Arch abstraction refactor — traits + `cfg` to split x86_64 code out.
29. ARM64 vCPU on KVM — `KVM_ARM_VCPU_INIT`, `KVM_SET_ONE_REG`, bare-metal hello guest.
30. Hand-written FDT generator — boot an arm64 Linux `Image`.
31. ARM devices — vGICv3, arch timer, PSCI SMP, PL011 UART; reuse virtio-mmio devices.
32. Hypervisor.framework vs KVM theory.
33. Apple Silicon backend — second hypervisor backend behind a trait; boot the arm64 guest on macOS.
