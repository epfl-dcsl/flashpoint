#!/bin/sh

set -eu

usage() {
    echo "Usage: $0 LOG_FILE" >&2
}

if [ "$#" -ne 1 ]; then
    usage
    exit 2
fi

log_file=$1

if [ ! -r "$log_file" ]; then
    echo "error: cannot read log file: $log_file" >&2
    exit 1
fi

LC_ALL=C awk '
function counter(line, name, value) {
    value = line
    sub("^.*" name "[[:space:]]*:[[:space:]]*", "", value)
    sub("[^0-9].*$", "", value)
    if (value !~ /^[0-9]+$/) {
        return -1
    }
    return value + 0
}

function record_checkpoint(label, cycles, instructions) {
    if (index($0, label) == 0) {
        return
    }

    cycles = counter($0, "mcycle")
    instructions = counter($0, "minstret")
    if (cycles >= 0 && instructions >= 0) {
        checkpoint_cycles[label] = cycles
        checkpoint_instructions[label] = instructions
        checkpoint_found[label] = 1
    }
}

function require_checkpoint(label) {
    if (!checkpoint_found[label]) {
        printf "error: missing anchor checkpoint: %s\n", label > "/dev/stderr"
        errors = 1
    }
}

function require_nonnegative(label, value) {
    if (value < 0) {
        printf "error: negative %s count; the log checkpoints are inconsistent\n", label > "/dev/stderr"
        errors = 1
    }
}

index($0, "[INFO | anchor]") {
    record_checkpoint("before srtm")
    record_checkpoint("after tpm_drv_entry_srtm")
    record_checkpoint("after srtm")
    record_checkpoint("before untrusted")
    record_checkpoint("after untrusted")
    record_checkpoint("before drtm")
    record_checkpoint("after tpm_drv_entry_drtm")
    record_checkpoint("after drtm")
    record_checkpoint("before sm")
}

index($0, "[INFO | tyche::riscv]") && $0 ~ /HartID:[[:space:]]*0([[:space:]]|$)/ {
    total_cycles = counter($0, "Mcycle")

    total_instructions_text = $0
    sub("^.*Minstret[[:space:]]*:?[[:space:]]*", "", total_instructions_text)
    sub("[^0-9].*$", "", total_instructions_text)
    if (total_instructions_text ~ /^[0-9]+$/) {
        total_instructions = total_instructions_text + 0
    } else {
        total_instructions = -1
    }

    if (total_cycles > 0 && total_instructions > 0) {
        total_found = 1
    }
}

END {
    if (!total_found) {
        print "error: missing positive hart 0 Mcycle/Minstret totals" > "/dev/stderr"
        errors = 1
    }

    require_checkpoint("before srtm")
    require_checkpoint("after tpm_drv_entry_srtm")
    require_checkpoint("after srtm")
    require_checkpoint("before untrusted")
    require_checkpoint("after untrusted")
    require_checkpoint("before drtm")
    require_checkpoint("after tpm_drv_entry_drtm")
    require_checkpoint("after drtm")
    require_checkpoint("before sm")

    if (errors) {
        exit 1
    }

    anchor_before_srtm_cycles = checkpoint_cycles["before srtm"]
    anchor_before_srtm_instructions = checkpoint_instructions["before srtm"]
    anchor_after_srtm_cycles = checkpoint_cycles["before untrusted"] - checkpoint_cycles["after srtm"]
    anchor_after_srtm_instructions = checkpoint_instructions["before untrusted"] - checkpoint_instructions["after srtm"]
    anchor_after_untrusted_cycles = checkpoint_cycles["before drtm"] - checkpoint_cycles["after untrusted"]
    anchor_after_untrusted_instructions = checkpoint_instructions["before drtm"] - checkpoint_instructions["after untrusted"]
    anchor_after_drtm_cycles = checkpoint_cycles["before sm"] - checkpoint_cycles["after drtm"]
    anchor_after_drtm_instructions = checkpoint_instructions["before sm"] - checkpoint_instructions["after drtm"]

    srtm_cycles = checkpoint_cycles["after srtm"] - checkpoint_cycles["after tpm_drv_entry_srtm"]
    srtm_instructions = checkpoint_instructions["after srtm"] - checkpoint_instructions["after tpm_drv_entry_srtm"]
    untrusted_cycles = checkpoint_cycles["after untrusted"] - checkpoint_cycles["before untrusted"]
    untrusted_instructions = checkpoint_instructions["after untrusted"] - checkpoint_instructions["before untrusted"]
    drtm_cycles = checkpoint_cycles["after drtm"] - checkpoint_cycles["after tpm_drv_entry_drtm"]
    drtm_instructions = checkpoint_instructions["after drtm"] - checkpoint_instructions["after tpm_drv_entry_drtm"]
    tyche_cycles = total_cycles - checkpoint_cycles["before sm"]
    tyche_instructions = total_instructions - checkpoint_instructions["before sm"]

    require_nonnegative("Anchor before SRTM mcycle", anchor_before_srtm_cycles)
    require_nonnegative("Anchor before SRTM minstret", anchor_before_srtm_instructions)
    require_nonnegative("Anchor after SRTM mcycle", anchor_after_srtm_cycles)
    require_nonnegative("Anchor after SRTM minstret", anchor_after_srtm_instructions)
    require_nonnegative("Anchor after untrusted firmware mcycle", anchor_after_untrusted_cycles)
    require_nonnegative("Anchor after untrusted firmware minstret", anchor_after_untrusted_instructions)
    require_nonnegative("Anchor after DRTM mcycle", anchor_after_drtm_cycles)
    require_nonnegative("Anchor after DRTM minstret", anchor_after_drtm_instructions)
    require_nonnegative("TPM driver (SRTM) mcycle", srtm_cycles)
    require_nonnegative("TPM driver (SRTM) minstret", srtm_instructions)
    require_nonnegative("untrusted firmware mcycle", untrusted_cycles)
    require_nonnegative("untrusted firmware minstret", untrusted_instructions)
    require_nonnegative("TPM driver (DRTM) mcycle", drtm_cycles)
    require_nonnegative("TPM driver (DRTM) minstret", drtm_instructions)
    require_nonnegative("Tyche mcycle", tyche_cycles)
    require_nonnegative("Tyche minstret", tyche_instructions)
    if (errors) {
        exit 1
    }

    anchor_cycles = anchor_before_srtm_cycles + anchor_after_srtm_cycles + anchor_after_untrusted_cycles + anchor_after_drtm_cycles
    anchor_instructions = anchor_before_srtm_instructions + anchor_after_srtm_instructions + anchor_after_untrusted_instructions + anchor_after_drtm_instructions

    printf "%-25s %12s %12s\n", "Region", "% mcycle", "% minstret"
    printf "%-25s %11.6f%% %11.6f%%\n", "Anchor", 100 * anchor_cycles / total_cycles, 100 * anchor_instructions / total_instructions
    printf "%-25s %11.6f%% %11.6f%%\n", "TPM driver (SRTM)", 100 * srtm_cycles / total_cycles, 100 * srtm_instructions / total_instructions
    printf "%-25s %11.6f%% %11.6f%%\n", "Untrusted firmware", 100 * untrusted_cycles / total_cycles, 100 * untrusted_instructions / total_instructions
    printf "%-25s %11.6f%% %11.6f%%\n", "TPM driver (DRTM)", 100 * drtm_cycles / total_cycles, 100 * drtm_instructions / total_instructions
    printf "%-25s %11.6f%% %11.6f%%\n", "Tyche", 100 * tyche_cycles / total_cycles, 100 * tyche_instructions / total_instructions
}
' "$log_file"
