# Lesson 1 — Hypervisor Theory

## Goal

By the end of this lesson we have:

- a clear mental model of what the CPU, KVM (in the kernel) and a VMM (in userspace) each do,
- a picture of how Firecracker and Cloud Hypervisor are put together, and why they differ,
- `docs/design.md`: Skiff's design doc. It covers the module boundaries, the thread
  model, the guest memory layout and the rules we follow (unsafe, errors, tests).
  Later lessons point back to it.
- a small refactor that splits `skiff` into a library and a thin binary, because the design
  depends on it (see "Decisions to review").

Apart from that split there is almost no code. Most of this lesson is reading and
deciding. Everything from Lesson 2 onwards depends on the vocabulary introduced here.

> **Who does what** (per our "you write, I coach" mode): you write the lib/bin split
> yourself — I give the target shape and explain the crate concepts below, you make the
> edits. The design doc is a decisions document; I'll draft a proposal from the section
> outline here and you edit/approve it, since the choices in it are yours to make.

## Background

### 1. What a CPU must offer to be virtualizable

A guest OS expects to own the machine. It writes to control registers, programs page tables,
disables interrupts and talks to devices. A hypervisor has to let the guest *believe* it
does all of this while keeping real control.

The classic approach is **trap-and-emulate**. The guest runs directly on the CPU at reduced
privilege. Every *sensitive* instruction (one that reads or changes machine state) traps to
the hypervisor, which emulates it and resumes the guest. Popek and Goldberg (1974) showed
this works only if every sensitive instruction is also *privileged*, meaning it traps when
run outside ring 0.

Original x86 failed this test. For example, `popf` silently ignores the interrupt flag
in user mode instead of trapping, and `sgdt` reveals the real GDT. Before 2005, x86
hypervisors worked around this with binary translation (VMware) or by modifying the guest
(Xen paravirtualization).

### 2. Intel VT-x (and AMD-V)

VT-x fixes the problem in hardware by adding a new dimension next to the rings:

- **VMX root mode**, where the hypervisor runs (host kernel + KVM).
- **VMX non-root mode**, where the guest runs. It has its own full set of rings 0–3, so the
  guest kernel really is in ring 0, but certain events cause a **VM exit** to root mode.

The switch is controlled by the **VMCS** (Virtual Machine Control Structure), a per-vCPU
memory region holding:

- guest state (registers, segment descriptors, CR0/CR3/CR4, …), loaded on **VM entry**,
- host state, restored on **VM exit**,
- execution controls: *which* events cause exits (I/O to certain ports, `hlt`, `cpuid`,
  certain MSRs, external interrupts, EPT violations, …),
- exit information: *why* we exited (the exit reason and qualification).

`vmlaunch`/`vmresume` enter the guest. An exit returns to the host with the reason recorded
in the VMCS. This is the loop at the heart of every hypervisor.

**EPT (Extended Page Tables)** handles memory. The guest has its own page tables that map
guest-virtual → guest-physical addresses. EPT, owned by the hypervisor, adds a second stage
that maps guest-physical → host-physical. The guest can manage its page tables without
exits, and it can only reach memory the hypervisor mapped for it. A guest-physical address
with no EPT mapping causes an **EPT violation** exit. That is how MMIO devices get emulated.

**Unrestricted guest** lets the guest run in real mode and in unpaged protected mode
natively. We rely on it in Lesson 3, where our first guest is 16-bit real-mode code.

### 3. KVM vs the VMM — who does what

KVM turns Linux into a hypervisor. It is not a complete one: it deals only with the CPU and
memory virtualization that has to happen in the kernel. Everything else is left to a
userspace **VMM** (Virtual Machine Monitor), such as QEMU, Firecracker, Cloud Hypervisor or
Skiff.

| KVM (kernel) | VMM (userspace, us) |
|---|---|
| VMXON, VMCS management, `vmlaunch`/`vmresume` | decide how many vCPUs and how much RAM |
| EPT: builds the second-stage tables from memory slots we give it | allocate guest RAM with `mmap` and register it as memory slots |
| handles most exits itself (EPT faults on RAM, many MSRs, `cpuid` from the table we set, …) | set initial vCPU state: registers, special registers, CPUID table, MSRs |
| optional in-kernel devices: LAPIC, IOAPIC, PIC, PIT (`KVM_CREATE_IRQCHIP`, `KVM_CREATE_PIT2`) | load the kernel and initramfs into guest memory, write boot params and the cmdline |
| injects interrupts into the guest | emulate every other device: serial, virtio, … |
| schedules vCPUs: a vCPU is just a thread blocked in `ioctl(KVM_RUN)` | run the event loop, the API, the sandboxing (jailer, seccomp) |
| dirty page tracking, fast paths (ioeventfd/irqfd) | handle exits KVM hands back: port I/O, MMIO, shutdown, … |

What matters for us: **we never touch VMX instructions**. Our whole interface to the
hardware is a set of `ioctl`s on file descriptors.

### 4. The shape of the KVM API

```
open("/dev/kvm")            ── system fd ── KVM_GET_API_VERSION, KVM_CHECK_EXTENSION,
       │                                    KVM_GET_SUPPORTED_CPUID, KVM_GET_VCPU_MMAP_SIZE
       │ KVM_CREATE_VM
       ▼
    VM fd ──────────────────────────────── KVM_SET_USER_MEMORY_REGION, KVM_CREATE_IRQCHIP,
       │                                    KVM_IRQFD, KVM_IOEVENTFD, …
       │ KVM_CREATE_VCPU (one per vCPU)
       ▼
   vCPU fd ─────────────────────────────── KVM_GET/SET_REGS, KVM_GET/SET_SREGS,
       │                                    KVM_SET_CPUID2, KVM_SET_MSRS, KVM_RUN
       │ mmap(vcpu_fd)
       ▼
   struct kvm_run  (shared page: exit_reason + per-exit data, e.g. io.port, mmio.phys_addr)
```

Each level's fd is the handle for the next. In Rust this maps onto owned types (`Kvm` →
`Vm` → `Vcpu`). Each owns its fd and closes it on drop, and each can only be created from
its parent. That is the "ownership mirrors the kernel object tree" idea we build on in
Lesson 2.

### 5. Life of one guest I/O instruction

Suppose the guest runs `out 0x3f8, al`, which writes one byte to the serial port:

1. The CPU is in non-root mode. The VMCS says I/O to this port exits, so a **VM exit** happens.
2. KVM's exit handler sees an I/O exit. No in-kernel device claims port 0x3f8, so KVM fills
   `kvm_run.exit_reason = KVM_EXIT_IO`, records the port, size and direction, and puts the
   data in the shared page.
3. `ioctl(KVM_RUN)` **returns** to our vCPU thread in userspace.
4. We look at the exit and hand the byte to our UART emulation, which prints it.
5. We call `ioctl(KVM_RUN)` again. KVM does a VM entry and the guest continues after the `out`.

That round trip costs thousands of cycles, and more under nested virtualization (WSL2).
Much of a VMM's design goes into avoiding it:

- **Virtio** (Part III) makes devices talk through shared-memory queues and exit only to say
  "there is work".
- **ioeventfd** turns that notification into an eventfd signal handled inside the kernel,
  without a trip through our `KVM_RUN` loop.
- **irqfd** lets us inject an interrupt by writing to an eventfd.

### 6. Firecracker

Built by AWS for Lambda and Fargate: thousands of short-lived, untrusted microVMs per host.
Its design follows from two goals, **small attack surface** and **fast boot** (~125 ms to
guest init).

- **One process per VM.** Threads: an API thread (REST over a Unix socket), a VMM thread
  (epoll event loop that runs device emulation) and one thread per vCPU.
- **Minimal device model.** It has virtio-block, virtio-net, virtio-vsock, virtio-balloon,
  virtio-rng, a serial port, and an i8042 whose only job is to let the guest reset the
  machine. Historically there was no PCI, only virtio-mmio, and devices are described to the
  guest on the kernel command line. There is no BIOS or UEFI.
- **Direct kernel boot.** The VMM loads an uncompressed `vmlinux` ELF (or a `bzImage`)
  straight into guest memory, writes `boot_params`, and starts the vCPU in 64-bit mode at
  the kernel entry point. This is what we do in Lesson 6.
- **Defence in depth.** A separate `jailer` binary sets up cgroups, namespaces and chroot,
  and drops privileges before exec'ing Firecracker. Each thread gets its own seccomp filter.
- Rate limiters on I/O, and snapshot/restore.

### 7. Cloud Hypervisor

Started by Intel (now a Linux Foundation project) for general cloud workloads. It shares
Firecracker's rust-vmm roots but targets a wider scope:

- **PCI** (virtio-pci), **VFIO** device passthrough, CPU/memory/device **hotplug**, ACPI tables.
- Firmware boot (Rust hypervisor firmware, or UEFI via OVMF) as well as direct kernel boot.
  It can run Windows guests.
- **vhost-user**: devices can run in separate processes.
- x86_64 and aarch64, with KVM and Microsoft's MSHV as backends.

Roughly: Firecracker keeps the device model as small as it can, and Cloud Hypervisor keeps
it small and modern but complete. Skiff starts on the Firecracker end and moves toward Cloud
Hypervisor only where a lesson needs it (PCI in Part V for GPU work).

### 8. rust-vmm

This is the set of crates the two projects share: `kvm-ioctls`/`kvm-bindings`, `vm-memory`,
`linux-loader`, `vm-superio`, `virtio-queue`, `event-manager`, `vmm-sys-util`, … . Each one
corresponds to something we will hand-write. Part VI swaps ours for theirs, and we keep our
tests to check that nothing breaks.

### 9. (For contrast) QEMU

QEMU is a general emulator that can use KVM for acceleration. It has hundreds of emulated
devices, is written in C, and boots through firmware by default. It is the reference for
"what does real hardware do", and its source is often the best documentation for device
quirks. It is also the reason microVM projects exist: less code means less attack surface.

## Steps

### 1. Read (student)

Suggested reading, roughly in priority order:

1. [KVM API documentation](https://docs.kernel.org/virt/kvm/api.html). Skim sections 1–4 and
   read the parts on `KVM_CREATE_VM`, `KVM_SET_USER_MEMORY_REGION`, `KVM_RUN` and `struct kvm_run`.
2. [Using the KVM API](https://lwn.net/Articles/658511/) (LWN, 2015). It is a whole tiny VMM in
   C, and our Lessons 2–4 follow it closely.
3. [Firecracker design doc](https://github.com/firecracker-microvm/firecracker/blob/main/docs/design.md).
4. Firecracker NSDI'20 paper, *"Firecracker: Lightweight Virtualization for Serverless
   Applications"*. Sections 3 and 4 are enough.
5. Intel SDM Vol. 3C, chapters 24–26 (VMX) — reference only, dip in when curious.

### 2. Explore your own machine (student)

```sh
# Which VT-x features does the (nested) CPU expose?
grep -m1 'vmx flags' /proc/cpuinfo
# How is KVM configured? We rely on ept=Y and unrestricted_guest=Y.
for p in ept unrestricted_guest nested vpid enable_apicv; do
  echo "$p=$(cat /sys/module/kvm_intel/parameters/$p)"
done
```

On the current dev box: `ept=Y unrestricted_guest=Y nested=Y vpid=Y enable_apicv=N`.
APICv is not available under Hyper-V nesting, so interrupt delivery goes through extra exits.
This is one more reason to expect WSL2 to be slower than bare metal.

### 3. Write `docs/design.md` (implementation)

The design doc is short and opinionated, and it changes as lessons land. Proposed sections:

1. **Goals and non-goals.** Learning over completeness. Direct kernel boot only. No
   firmware, live migration or Windows guests.
2. **Process and thread model, and how it grows.**
   - Lessons 3–9: single thread, with `main` running the one vCPU's `KVM_RUN` loop.
   - Lesson 10: one thread per vCPU, plus the main thread.
   - Lesson 12: a VMM thread with our own epoll event loop owns the devices, and vCPU
     threads kick it through ioeventfd.
   - Lesson 17: an API thread.
3. **Module map and boundaries** (to become `skiff/src/…` as lessons add them):

   | Module | Responsibility | Arrives in | Part VI replacement |
   |---|---|---|---|
   | `kvm` | raw ioctl numbers and `#[repr(C)]` structs; safe `Kvm`/`Vm`/`Vcpu` wrappers | L2 | `kvm-ioctls`, `kvm-bindings` |
   | `memory` | `GuestMemory`: guest RAM regions, bounds-checked reads/writes by guest-physical address | L3 | `vm-memory` |
   | `vcpu` | the run loop and exit dispatch | L4 | — |
   | `devices` | `Bus` (PIO and MMIO address → device), device implementations | L4, L7 | `vm-device`, `vm-superio` |
   | `arch::x86_64` | page tables, GDT, registers, `boot_params`, E820, MP tables | L5, L6, L8 | `linux-loader` (partly) |
   | `loader` | ELF / bzImage parsing | L6 | `linux-loader` |
   | `virtio` | virtio-mmio transport, split virtqueue, devices | L11+ | `virtio-queue`, `virtio-device` |
   | `event` | epoll loop, eventfd | L12 | `event-manager`, `vmm-sys-util` |

   The rule is that each module's public API is what its tests use. In Part VI we swap the
   implementation behind it and keep the tests.
4. **Guest physical memory layout (x86_64).** This is a proposal, with numbers borrowed from
   Firecracker, and Lesson 6 fixes the details:
   - `0x0000_0000`: RAM starts. Low memory holds boot structures: GDT, page tables,
     `boot_params` ("zero page") at `0x7000`, cmdline at `0x2_0000`.
   - `0x10_0000` (1 MiB): kernel load address.
   - RAM continues up to `min(ram_size, 3 GiB)`.
   - `0xC000_0000`–`0xFFFF_FFFF`: 32-bit MMIO gap (virtio-mmio devices, IOAPIC at `0xFEC0_0000`,
     LAPIC at `0xFEE0_0000`).
   - `4 GiB`+: the rest of RAM, if more than 3 GiB is configured.
5. **Rules we follow.**
   - *Unsafe:* every `unsafe` block is as small as possible and has a `// SAFETY:` comment
     (already enforced by our lints). Raw pointers into guest memory never leave the
     `memory` module.
   - *Errors:* one error enum per module. `main` prints the chain.
   - *Logging:* see the decisions below.
   - *Tests:* pure logic gets plain unit tests. Tests that need `/dev/kvm` are `#[ignore]`
     and run with `cargo test -- --ignored`. Guest-level tests (boot a guest, expect output
     on serial) come as integration tests from Lesson 3.
   - *Arch-specific code:* x86 code goes only in `arch::x86_64`, and KVM calls go only
     through `kvm`. There are no traits for these until a second implementation exists
     (Part VII).

### 4. Split the crate into lib + bin (implementation)

- Move the KVM check from `main.rs` into `skiff/src/lib.rs`, unchanged in behaviour. It
  becomes the seed of the `kvm` module in Lesson 2.
- `main.rs` becomes a thin wrapper that calls the library and turns the result into an
  exit code.
- Reason: integration tests in `skiff/tests/` (and, in Part VI, comparison tests) can only
  use a library crate's public API, not a binary's.

Rust concepts: library vs binary crate targets in one package, `pub` visibility, and the
crate root (`lib.rs`) vs a binary using the library by its crate name (`use skiff::…`).

### 5. Housekeeping (implementation)

- Mark Lesson 0 ✅ and Lesson 1 🚧 in `lessons/README.md`, and link this file.
- Update the `README.md` status line and link to `docs/design.md`.

## Decisions to review

These choices shape later lessons. Recommendations first:

1. **Lib + bin split now** (recommended), or wait until the first integration test in Lesson 3.
   It is cheap now and gets harder once modules exist.
2. **Error handling.** (a) Hand-written error enums with `impl Display`/`Error`, like
   `KvmUnavailable` in Lesson 0 (recommended for Part I, since it teaches the traits). Or
   (b) adopt `thiserror` now to cut the boilerplate. Either way we avoid `anyhow` inside
   the library.
3. **Logging.** Add `log` + `env_logger` in Lesson 3, when exits become worth tracing
   (recommended). Or stay with `eprintln!`.
4. **Memory layout numbers.** Keep the Firecracker-like proposal above, or pick something
   different on purpose.

## How to test

There is not much to execute in this lesson:

| Check | Command | Expected |
|---|---|---|
| Refactor didn't change behaviour | `cargo run` | `KVM is available at /dev/kvm` |
| Tests still pass | `cargo test` | all pass, now from the lib target |
| Lints | `cargo clippy --all-targets -- -D warnings` | clean |
| CI | GitHub → Actions | green |

### Self-check questions

You should be able to answer these without looking back:

1. Why could original x86 not be virtualized with pure trap-and-emulate? Name one offending instruction.
2. What is the difference between VMX root/non-root and rings 0–3?
3. A guest reads guest-physical address `0xD000_0000`, and no memory slot covers it. Walk
   through what happens, all the way to our code.
4. Why is a vCPU "just a thread" from the host kernel's point of view?
5. Which of these does KVM do, and which does the VMM do: set the CPUID table, apply the
   CPUID table when the guest runs `cpuid`, load the kernel image, build EPT entries,
   emulate a 16550 UART?
6. Why does Firecracker have no BIOS, and what does it do instead?
7. What does ioeventfd save compared with a plain `KVM_EXIT_MMIO`?
