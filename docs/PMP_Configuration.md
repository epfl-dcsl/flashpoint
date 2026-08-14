# FlashPoint PMP configuration through the boot sequence

This document provides an explanation of the PMP configurations in Flashpoint, at different stages of Anchor execution, using the logs recorded from a boot on QEMU.

Every PMP log line below is from the [`expected_results/pmp_logs_figure_4.txt`](../expected_results/pmp_logs_figure_4.txt). 

Text, tables, and diagrams
around the logs explain which software or device each active entry protects and
how the FlashPoint transition instructions change enforcement.

The configuration is defined in [`anchor/src/main.rs`](../anchor/src/main.rs)
and constructed in [`anchor/src/pmp_static.rs`](../anchor/src/pmp_static.rs).
The QEMU implementation of `exit_anchor` and `enter_anchor` is in
[`qemu/target/riscv/flashpoint.c`](../qemu/target/riscv/flashpoint.c).

## How to read a PMP line

For example:

```text
[DEBUG | anchor] hart 0 - PMP  0          80000000         80080000 | R__  NAPOT
```

The two addresses are the inclusive start and exclusive end of the decoded
range. `R__` means read is allowed but write and execute are denied. `L` means
the PMP entry is locked. `NAPOT` is the naturally aligned power-of-two address
matching mode. An `OFF` entry is inactive.

Flashpoint is built upon the default PMP semantics with M-mode software. 
Consequently,
an active entry without `L` does not restrict M-mode access. 
Locking the entry makes its permissions apply to the M-mode software.
An unmatched M-mode address remains accessible; therefore each domain lists
the exceptional ranges it needs to allow or deny rather than an exhaustive
allowlist.

## Memory map used by the active entries

```text
0x04000000 +------------------------------+
           | TPM TIS MMIO                 |  0x04000000..0x04008000
0x04008000 +------------------------------+

0x80000000 +------------------------------+
           | Anchor                       |  0x80000000..0x80080000
0x80080000 +------------------------------+
           | TPM driver                   |  0x80080000..0x80100000
0x80100000 +------------------------------+
           | Gap                          |
0x80200000 +------------------------------+
           | Untrusted firmware memory    |  0x80200000..0x80400000
0x80400000 +------------------------------+
           | Security-monitor PMP region  |  0x80400000..0x80500000
           | Tyche start: 0x80450000      |
0x80500000 +------------------------------+
```

## The changing `L` bit

Notice the transition from Anchor to TPM driver on Hart 0 changes the `L` bit of the highest priority PMP entry (PMPCFG0). 
This change occurs via the "exit\_anchor" instruction. 
The Anchor's own execution is not disrupted (since L = 0 for its own memory region, right before the execution of the "exit\_anchor" instruction). 

The other PMP entries are installed with `L=1` immediately by the Anchor.

```text
Anchor configures a domain       exit_anchor executes       Target runs

PMP 0: range + permissions, L=0  ------------------------>  same PMP 0, L=1
       anchor may still execute       sets PMP0.L             permissions now
       and finish the transition                               constrain M-mode

Target finishes                  enter_anchor executes      Anchor resumes

locked PMP entries, L=1          ------------------------>  all L bits cleared;
                                      clears L bits          anchor installs the
                                                            next domain table
```

The clearest instance occurs around SRTM:

```text
Before exit_anchor:  PMP 0 | R__  NAPOT   (anchor log)
                                  |
                                  | exit_anchor sets PMP0.L
                                  v
Inside TPM driver:   PMP 0 | R__L NAPOT   (TPM-driver log)
```

The address and R/W/X permissions do not change during that instruction; only
the lock bit changes.

## PMP Snapshots annotated with Phases and Explanations

=== PMP snapshots by boot phase ===

### Reset | Hart 0

```text
[DEBUG | anchor] hart 0 - PMP  0                 0                0 | ___  OFF
[DEBUG | anchor] hart 0 - PMP  1                 0                0 | ___  OFF
[DEBUG | anchor] hart 0 - PMP  2                 0                0 | ___  OFF
[DEBUG | anchor] hart 0 - PMP  3                 0                0 | ___  OFF
[DEBUG | anchor] hart 0 - PMP  4                 0                0 | ___  OFF
[DEBUG | anchor] hart 0 - PMP  5                 0                0 | ___  OFF
[DEBUG | anchor] hart 0 - PMP  6                 0                0 | ___  OFF
[DEBUG | anchor] hart 0 - PMP  7                 0                0 | ___  OFF
[DEBUG | anchor] hart 0 - PMP  8                 0                0 | ___  OFF
[DEBUG | anchor] hart 0 - PMP  9                 0                0 | ___  OFF
[DEBUG | anchor] hart 0 - PMP 10                 0                0 | ___  OFF
[DEBUG | anchor] hart 0 - PMP 11                 0                0 | ___  OFF
[DEBUG | anchor] hart 0 - PMP 12                 0                0 | ___  OFF
[DEBUG | anchor] hart 0 - PMP 13                 0                0 | ___  OFF
[DEBUG | anchor] hart 0 - PMP 14                 0                0 | ___  OFF
[DEBUG | anchor] hart 0 - PMP 15                 0                0 | ___  OFF
```

All entries are inactive immediately after reset, so PMP imposes no restriction on hart 0 yet.

### Reset | Hart 1

```text
[DEBUG | anchor] hart 1 - PMP  0                 0                0 | ___  OFF
[DEBUG | anchor] hart 1 - PMP  1                 0                0 | ___  OFF
[DEBUG | anchor] hart 1 - PMP  2                 0                0 | ___  OFF
[DEBUG | anchor] hart 1 - PMP  3                 0                0 | ___  OFF
[DEBUG | anchor] hart 1 - PMP  4                 0                0 | ___  OFF
[DEBUG | anchor] hart 1 - PMP  5                 0                0 | ___  OFF
[DEBUG | anchor] hart 1 - PMP  6                 0                0 | ___  OFF
[DEBUG | anchor] hart 1 - PMP  7                 0                0 | ___  OFF
[DEBUG | anchor] hart 1 - PMP  8                 0                0 | ___  OFF
[DEBUG | anchor] hart 1 - PMP  9                 0                0 | ___  OFF
[DEBUG | anchor] hart 1 - PMP 10                 0                0 | ___  OFF
[DEBUG | anchor] hart 1 - PMP 11                 0                0 | ___  OFF
[DEBUG | anchor] hart 1 - PMP 12                 0                0 | ___  OFF
[DEBUG | anchor] hart 1 - PMP 13                 0                0 | ___  OFF
[DEBUG | anchor] hart 1 - PMP 14                 0                0 | ___  OFF
[DEBUG | anchor] hart 1 - PMP 15                 0                0 | ___  OFF
```

Hart 1 also starts with no active PMP entries. PMPs are per-hart.

### After initialization / before SRTM | Hart 0

```text
[DEBUG | anchor] hart 0 - PMP  0          80000000         80080000 | R__  NAPOT
[DEBUG | anchor] hart 0 - PMP  1          80200000         80400000 | ___L NAPOT
[DEBUG | anchor] hart 0 - PMP  2                 0                0 | ___  OFF
[DEBUG | anchor] hart 0 - PMP  3                 0                0 | ___  OFF
[DEBUG | anchor] hart 0 - PMP  4                 0                0 | ___  OFF
[DEBUG | anchor] hart 0 - PMP  5                 0                0 | ___  OFF
[DEBUG | anchor] hart 0 - PMP  6                 0                0 | ___  OFF
[DEBUG | anchor] hart 0 - PMP  7                 0                0 | ___  OFF
[DEBUG | anchor] hart 0 - PMP  8                 0                0 | ___  OFF
[DEBUG | anchor] hart 0 - PMP  9                 0                0 | ___  OFF
[DEBUG | anchor] hart 0 - PMP 10                 0                0 | ___  OFF
[DEBUG | anchor] hart 0 - PMP 11                 0                0 | ___  OFF
[DEBUG | anchor] hart 0 - PMP 12                 0                0 | ___  OFF
[DEBUG | anchor] hart 0 - PMP 13                 0                0 | ___  OFF
[DEBUG | anchor] hart 0 - PMP 14                 0                0 | ___  OFF
[DEBUG | anchor] hart 0 - PMP 15                 0                0 | ___  OFF
```

This is the `TPM_DRIVER_DOMAIN_SRTM` table:

| Entry | Range | Meaning |
|---|---|---|
| PMP 0 | Anchor, `0x80000000..0x80080000` | Grants the TPM driver read-only access so it can measure the anchor. It is not locked in this snapshot because the anchor is still executing. |
| PMP 1 | Untrusted firmware, `0x80200000..0x80400000` | Denies all access and is already locked, preventing the SRTM TPM driver from accessing untrusted memory. |

Next, hart 0 executes `exit_anchor`. That instruction sets `PMP0.L`; it does not alter the range or the `R__` permission.

### After initialization / before untrusted firmware | Hart 1

```text
[DEBUG | anchor] hart 1 - PMP  0          80000000         80100000 | ___  NAPOT
[DEBUG | anchor] hart 1 - PMP  1           4000000          4008000 | ___L NAPOT
[DEBUG | anchor] hart 1 - PMP  2                 0                0 | ___  OFF
[DEBUG | anchor] hart 1 - PMP  3                 0                0 | ___  OFF
[DEBUG | anchor] hart 1 - PMP  4                 0                0 | ___  OFF
[DEBUG | anchor] hart 1 - PMP  5                 0                0 | ___  OFF
[DEBUG | anchor] hart 1 - PMP  6                 0                0 | ___  OFF
[DEBUG | anchor] hart 1 - PMP  7                 0                0 | ___  OFF
[DEBUG | anchor] hart 1 - PMP  8                 0                0 | ___  OFF
[DEBUG | anchor] hart 1 - PMP  9                 0                0 | ___  OFF
[DEBUG | anchor] hart 1 - PMP 10                 0                0 | ___  OFF
[DEBUG | anchor] hart 1 - PMP 11                 0                0 | ___  OFF
[DEBUG | anchor] hart 1 - PMP 12                 0                0 | ___  OFF
[DEBUG | anchor] hart 1 - PMP 13                 0                0 | ___  OFF
[DEBUG | anchor] hart 1 - PMP 14                 0                0 | ___  OFF
[DEBUG | anchor] hart 1 - PMP 15                 0                0 | ___  OFF
```

Hart 1 skips SRTM and receives the `FIRMWARE_DOMAIN` table directly:

| Entry | Range | Meaning |
|---|---|---|
| PMP 0 | Anchor + TPM driver, `0x80000000..0x80100000` | Will deny the untrusted firmware access to both trusted components after `exit_anchor` locks the entry. |
| PMP 1 | TPM TIS MMIO, `0x04000000..0x04008000` | Already locked and denies the untrusted firmware direct access to the TPM device. |

### Inside TPM driver during SRTM | Hart 0

```text
[INFO | tpm-driver] hart 0 - PMP  0          80000000         80080000 | R__L NAPOT
[INFO | tpm-driver] hart 0 - PMP  1          80200000         80400000 | ___L NAPOT
[INFO | tpm-driver] hart 0 - PMP  2                 0                0 | ___  OFF
[INFO | tpm-driver] hart 0 - PMP  3                 0                0 | ___  OFF
[INFO | tpm-driver] hart 0 - PMP  4                 0                0 | ___  OFF
[INFO | tpm-driver] hart 0 - PMP  5                 0                0 | ___  OFF
[INFO | tpm-driver] hart 0 - PMP  6                 0                0 | ___  OFF
[INFO | tpm-driver] hart 0 - PMP  7                 0                0 | ___  OFF
[INFO | tpm-driver] hart 0 - PMP  8                 0                0 | ___  OFF
[INFO | tpm-driver] hart 0 - PMP  9                 0                0 | ___  OFF
[INFO | tpm-driver] hart 0 - PMP 10                 0                0 | ___  OFF
[INFO | tpm-driver] hart 0 - PMP 11                 0                0 | ___  OFF
[INFO | tpm-driver] hart 0 - PMP 12                 0                0 | ___  OFF
[INFO | tpm-driver] hart 0 - PMP 13                 0                0 | ___  OFF
[INFO | tpm-driver] hart 0 - PMP 14                 0                0 | ___  OFF
[INFO | tpm-driver] hart 0 - PMP 15                 0                0 | ___  OFF
```

The change in the Locked bit is visible:

```text
Anchor before SRTM       exit_anchor       TPM driver during SRTM
PMP 0: R__  NAPOT   -------------------->  PMP 0: R__L NAPOT
```

`L=1` is a consequence of executing `exit_anchor`. 
PMP 0 now constrains the M-mode TPM driver to read-only access to the anchor. 
PMP 1 continues to deny access to untrusted firmware memory. 
The TPM driver's own code/data and the TPM MMIO range do not match these entries and remain accessible to M-mode.

When SRTM finishes, `enter_anchor` clears the lock bits and returns control to the anchor, which replaces this table with the firmware table.

### After SRTM / before untrusted firmware | Hart 0

```text
[DEBUG | anchor] hart 0 - PMP  0          80000000         80100000 | ___  NAPOT
[DEBUG | anchor] hart 0 - PMP  1           4000000          4008000 | ___L NAPOT
[DEBUG | anchor] hart 0 - PMP  2                 0                0 | ___  OFF
[DEBUG | anchor] hart 0 - PMP  3                 0                0 | ___  OFF
[DEBUG | anchor] hart 0 - PMP  4                 0                0 | ___  OFF
[DEBUG | anchor] hart 0 - PMP  5                 0                0 | ___  OFF
[DEBUG | anchor] hart 0 - PMP  6                 0                0 | ___  OFF
[DEBUG | anchor] hart 0 - PMP  7                 0                0 | ___  OFF
[DEBUG | anchor] hart 0 - PMP  8                 0                0 | ___  OFF
[DEBUG | anchor] hart 0 - PMP  9                 0                0 | ___  OFF
[DEBUG | anchor] hart 0 - PMP 10                 0                0 | ___  OFF
[DEBUG | anchor] hart 0 - PMP 11                 0                0 | ___  OFF
[DEBUG | anchor] hart 0 - PMP 12                 0                0 | ___  OFF
[DEBUG | anchor] hart 0 - PMP 13                 0                0 | ___  OFF
[DEBUG | anchor] hart 0 - PMP 14                 0                0 | ___  OFF
[DEBUG | anchor] hart 0 - PMP 15                 0                0 | ___  OFF
```

This is the same firmware-domain configuration previously shown on hart 1.
PMP 0 covers and will protect both the anchor and TPM driver; PMP 1 protects the
TPM device. The anchor then executes `exit_anchor`, locking PMP 0 before jumping
to the untrusted firmware. The firmware-side `PMP0.L=1` state is inferred from
the instruction semantics; the untrusted firmware does not print a PMP dump.

### After untrusted firmware / before DRTM | Hart 0

```text
[DEBUG | anchor] hart 0 - PMP  0          80000000         80080000 | ___  NAPOT
[DEBUG | anchor] hart 0 - PMP  1          80400000         80500000 | R__L NAPOT
[DEBUG | anchor] hart 0 - PMP  2          80200000         80400000 | ___L NAPOT
[DEBUG | anchor] hart 0 - PMP  3                 0                0 | ___  OFF
[DEBUG | anchor] hart 0 - PMP  4                 0                0 | ___  OFF
[DEBUG | anchor] hart 0 - PMP  5                 0                0 | ___  OFF
[DEBUG | anchor] hart 0 - PMP  6                 0                0 | ___  OFF
[DEBUG | anchor] hart 0 - PMP  7                 0                0 | ___  OFF
[DEBUG | anchor] hart 0 - PMP  8                 0                0 | ___  OFF
[DEBUG | anchor] hart 0 - PMP  9                 0                0 | ___  OFF
[DEBUG | anchor] hart 0 - PMP 10                 0                0 | ___  OFF
[DEBUG | anchor] hart 0 - PMP 11                 0                0 | ___  OFF
[DEBUG | anchor] hart 0 - PMP 12                 0                0 | ___  OFF
[DEBUG | anchor] hart 0 - PMP 13                 0                0 | ___  OFF
[DEBUG | anchor] hart 0 - PMP 14                 0                0 | ___  OFF
[DEBUG | anchor] hart 0 - PMP 15                 0                0 | ___  OFF
```

After `enter_anchor` returns from the untrusted firmware, hart 0 installs the
`TPM_DRIVER_DOMAIN_DRTM` table:

| Entry | Range | Meaning |
|---|---|---|
| PMP 0 | Anchor, `0x80000000..0x80080000` | Will deny the DRTM TPM driver all access to anchor memory after `exit_anchor` sets `L`. |
| PMP 1 | Security monitor, `0x80400000..0x80500000` | Grants read-only access so the TPM driver can measure Tyche; already locked. |
| PMP 2 | Untrusted firmware, `0x80200000..0x80400000` | Prevents the DRTM TPM driver from reading, writing, or executing untrusted firmware memory; already locked. |

### Inside TPM driver during DRTM | Hart 0

```text
[INFO | tpm-driver] hart 0 - PMP  0          80000000         80080000 | ___L NAPOT
[INFO | tpm-driver] hart 0 - PMP  1          80400000         80500000 | R__L NAPOT
[INFO | tpm-driver] hart 0 - PMP  2          80200000         80400000 | ___L NAPOT
[INFO | tpm-driver] hart 0 - PMP  3                 0                0 | ___  OFF
[INFO | tpm-driver] hart 0 - PMP  4                 0                0 | ___  OFF
[INFO | tpm-driver] hart 0 - PMP  5                 0                0 | ___  OFF
[INFO | tpm-driver] hart 0 - PMP  6                 0                0 | ___  OFF
[INFO | tpm-driver] hart 0 - PMP  7                 0                0 | ___  OFF
[INFO | tpm-driver] hart 0 - PMP  8                 0                0 | ___  OFF
[INFO | tpm-driver] hart 0 - PMP  9                 0                0 | ___  OFF
[INFO | tpm-driver] hart 0 - PMP 10                 0                0 | ___  OFF
[INFO | tpm-driver] hart 0 - PMP 11                 0                0 | ___  OFF
[INFO | tpm-driver] hart 0 - PMP 12                 0                0 | ___  OFF
[INFO | tpm-driver] hart 0 - PMP 13                 0                0 | ___  OFF
[INFO | tpm-driver] hart 0 - PMP 14                 0                0 | ___  OFF
[INFO | tpm-driver] hart 0 - PMP 15                 0                0 | ___  OFF
```

Again, the first-entry lock transition is explicit:

```text
Anchor before DRTM       exit_anchor       TPM driver during DRTM
PMP 0: ___  NAPOT   -------------------->  PMP 0: ___L NAPOT
```

The DRTM TPM driver cannot access the anchor or untrusted firmware, can only
read the security-monitor region, and can access its own unmatched region and
the TPM device. This confines the software while it hashes Tyche into PCR 17.

### After untrusted firmware / before security monitor | Hart 1

```text
[DEBUG | anchor] hart 1 - PMP  0          80000000         80100000 | ___  NAPOT
[DEBUG | anchor] hart 1 - PMP  1           4000000          4008000 | ___L NAPOT
[DEBUG | anchor] hart 1 - PMP  2                 0                0 | ___  OFF
[DEBUG | anchor] hart 1 - PMP  3                 0                0 | ___  OFF
[DEBUG | anchor] hart 1 - PMP  4                 0                0 | ___  OFF
[DEBUG | anchor] hart 1 - PMP  5                 0                0 | ___  OFF
[DEBUG | anchor] hart 1 - PMP  6                 0                0 | ___  OFF
[DEBUG | anchor] hart 1 - PMP  7                 0                0 | ___  OFF
[DEBUG | anchor] hart 1 - PMP  8                 0                0 | ___  OFF
[DEBUG | anchor] hart 1 - PMP  9                 0                0 | ___  OFF
[DEBUG | anchor] hart 1 - PMP 10                 0                0 | ___  OFF
[DEBUG | anchor] hart 1 - PMP 11                 0                0 | ___  OFF
[DEBUG | anchor] hart 1 - PMP 12                 0                0 | ___  OFF
[DEBUG | anchor] hart 1 - PMP 13                 0                0 | ___  OFF
[DEBUG | anchor] hart 1 - PMP 14                 0                0 | ___  OFF
[DEBUG | anchor] hart 1 - PMP 15                 0                0 | ___  OFF
```

Hart 1 has returned from the untrusted firmware and installs the
`SECURITY_MONITOR_DOMAIN` table. It has the same two protected ranges as the
firmware domain:

- PMP 0 will deny the security monitor access to the anchor and TPM driver.
- PMP 1 already denies direct access to the TPM device.

After this snapshot, `exit_anchor` locks PMP 0 and transfers hart 1 to the
security monitor.

### After DRTM / before security monitor | Hart 0

```text
[DEBUG | anchor] hart 0 - PMP  0          80000000         80100000 | ___  NAPOT
[DEBUG | anchor] hart 0 - PMP  1           4000000          4008000 | ___L NAPOT
[DEBUG | anchor] hart 0 - PMP  2                 0                0 | ___  OFF
[DEBUG | anchor] hart 0 - PMP  3                 0                0 | ___  OFF
[DEBUG | anchor] hart 0 - PMP  4                 0                0 | ___  OFF
[DEBUG | anchor] hart 0 - PMP  5                 0                0 | ___  OFF
[DEBUG | anchor] hart 0 - PMP  6                 0                0 | ___  OFF
[DEBUG | anchor] hart 0 - PMP  7                 0                0 | ___  OFF
[DEBUG | anchor] hart 0 - PMP  8                 0                0 | ___  OFF
[DEBUG | anchor] hart 0 - PMP  9                 0                0 | ___  OFF
[DEBUG | anchor] hart 0 - PMP 10                 0                0 | ___  OFF
[DEBUG | anchor] hart 0 - PMP 11                 0                0 | ___  OFF
[DEBUG | anchor] hart 0 - PMP 12                 0                0 | ___  OFF
[DEBUG | anchor] hart 0 - PMP 13                 0                0 | ___  OFF
[DEBUG | anchor] hart 0 - PMP 14                 0                0 | ___  OFF
[DEBUG | anchor] hart 0 - PMP 15                 0                0 | ___  OFF
```

After `enter_anchor` returns from the DRTM TPM driver, hart 0 installs the same
security-monitor domain as hart 1. PMP 0 protects both the anchor and TPM driver
from the security monitor, while PMP 1 protects the TPM device. `exit_anchor`
then locks PMP 0 and enters the measured security monitor.

The final enforced configuration on both harts is therefore:

```text
Security monitor (M-mode)
        |
        +-- PMP 0: 0x80000000..0x80100000, ___L
        |          no access to Anchor or TPM driver
        |
        +-- PMP 1: 0x04000000..0x04008000, ___L
                   no direct access to the TPM device
```
