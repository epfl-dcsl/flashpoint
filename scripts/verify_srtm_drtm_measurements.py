#!/usr/bin/env python3

"""
Verify FlashPoint SRTM/DRTM measurements and the resulting TPM quote.

The verifier reconstructs the bytes visible in memory from the ELF PT_LOAD
segments.  This is important: it preserves zero-filled alignment gaps and uses
the exact addresses and lengths passed by the TPM driver, instead of hashing
whole sections or whole ELF files.

The public key is supplied in the same log as the quote.  Consequently this
checks the quote's cryptographic integrity, but it does not establish trust in
the attestation key.  A real remote verifier must additionally authenticate
the key (for example with an AK certificate) and supply a fresh quote nonce.
"""

from __future__ import annotations

import argparse
import hashlib
import hmac
import re
import struct
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Sequence


SHA384_SIZE = 48
RSA_KEY_SIZE = 384
TPM_GENERATED_VALUE = 0xFF544347
TPM_ST_ATTEST_QUOTE = 0x8018
TPM_ALG_SHA384 = 0x000C
PCR0 = 0
PCR17 = 17

# PMP_STATE_AFTER_INIT contains two RV64 harts, each represented by two
# pmpcfg CSRs followed by 16 pmpaddr CSRs.  These are the expected values for
# the QEMU run.  Values are laid out exactly like in target memory i.e. Hart 0 pmpcfgs, pmpaddrs, and Hart 1 pmpcfgs and pmpaddrs.
PMP_STATE: tuple[int, ...] = (
    # Hart 0
    0x9819, 0x0,
    0x2000FFFF, 0x200BFFFF,
    0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0,
    0x0, 0x0, 0x0, 0x0, 0x0, 0x0,
    # Hart 1
    0x9818, 0x0,
    0x2001FFFF, 0x1000FFF,
    0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0,
    0x0, 0x0, 0x0, 0x0, 0x0, 0x0,
)


class VerificationError(Exception):
    """An input cannot be parsed or is inconsistent with the expected format."""


@dataclass(frozen=True)
class LoadSegment:
    address: int
    file_size: int
    memory_size: int
    data: bytes


@dataclass(frozen=True)
class MeasurementRegion:
    name: str
    image: "ElfImage"
    address: int
    size: int

    def read(self) -> bytes:
        return self.image.read_memory(self.address, self.size)


@dataclass(frozen=True)
class PcrSelection:
    algorithm: int
    bitmap: bytes

    @property
    def indices(self) -> tuple[int, ...]:
        return tuple(
            byte_index * 8 + bit_index
            for byte_index, value in enumerate(self.bitmap)
            for bit_index in range(8)
            if value & (1 << bit_index)
        )


@dataclass(frozen=True)
class Attestation:
    raw: bytes
    magic: int
    attest_type: int
    qualified_signer: bytes
    extra_data: bytes
    selections: tuple[PcrSelection, ...]
    pcr_digest: bytes


@dataclass(frozen=True)
class LogValues:
    pcr0: bytes
    pcr17: bytes
    drtm_address: int
    drtm_size: int
    attestation_buffer: bytes
    modulus: bytes
    signature: bytes


class ElfImage:
    """A minimal ELF64 little-endian PT_LOAD reader."""

    _ELF_HEADER = struct.Struct("<16sHHIQQQIHHHHHH")
    _PROGRAM_HEADER = struct.Struct("<IIQQQQQQ")

    def __init__(self, path: Path):
        self.path = path
        try:
            elf = path.read_bytes()
        except OSError as error:
            raise VerificationError(f"cannot read ELF {path}: {error}") from error

        if len(elf) < self._ELF_HEADER.size:
            raise VerificationError(f"{path} is too short to be an ELF file")

        header = self._ELF_HEADER.unpack_from(elf)
        ident = header[0]
        if ident[:4] != b"\x7fELF":
            raise VerificationError(f"{path} is not an ELF file")
        if ident[4] != 2 or ident[5] != 1:
            raise VerificationError(
                f"{path} is not a 64-bit little-endian ELF file"
            )

        program_offset = header[5]
        program_entry_size = header[9]
        program_count = header[10]
        if program_entry_size < self._PROGRAM_HEADER.size:
            raise VerificationError(f"{path} has invalid program headers")

        segments: list[LoadSegment] = []
        for index in range(program_count):
            offset = program_offset + index * program_entry_size
            if offset + self._PROGRAM_HEADER.size > len(elf):
                raise VerificationError(f"{path} has a truncated program header table")
            (
                segment_type,
                _flags,
                file_offset,
                virtual_address,
                physical_address,
                file_size,
                memory_size,
                _alignment,
            ) = self._PROGRAM_HEADER.unpack_from(elf, offset)
            if segment_type != 1:  # PT_LOAD
                continue
            if file_size > memory_size or file_offset + file_size > len(elf):
                raise VerificationError(f"{path} has an invalid PT_LOAD segment")
            address = physical_address or virtual_address
            segments.append(
                LoadSegment(
                    address=address,
                    file_size=file_size,
                    memory_size=memory_size,
                    data=elf[file_offset : file_offset + file_size],
                )
            )

        if not segments:
            raise VerificationError(f"{path} has no PT_LOAD segments")
        self.segments = tuple(segments)

    def read_memory(self, address: int, size: int) -> bytes:
        """Return a loaded memory range, including zero-filled holes/BSS."""
        if address < 0 or size < 0:
            raise VerificationError("negative memory address or measurement size")
        end = address + size
        if end < address:
            raise VerificationError("measurement address overflow")

        image_start = min(segment.address for segment in self.segments)
        image_end = max(segment.address + segment.memory_size for segment in self.segments)
        if address < image_start or end > image_end:
            raise VerificationError(
                f"range 0x{address:x}..0x{end:x} is outside the loaded image "
                f"{self.path} (0x{image_start:x}..0x{image_end:x})"
            )

        result = bytearray(size)
        for segment in self.segments:
            overlap_start = max(address, segment.address)
            overlap_end = min(end, segment.address + segment.file_size)
            if overlap_start >= overlap_end:
                continue
            source_start = overlap_start - segment.address
            target_start = overlap_start - address
            length = overlap_end - overlap_start
            result[target_start : target_start + length] = segment.data[
                source_start : source_start + length
            ]
        return bytes(result)


class BigEndianReader:
    def __init__(self, data: bytes):
        self.data = data
        self.offset = 0

    def take(self, size: int, description: str) -> bytes:
        end = self.offset + size
        if size < 0 or end > len(self.data):
            raise VerificationError(f"truncated TPM attestation at {description}")
        value = self.data[self.offset:end]
        self.offset = end
        return value

    def integer(self, size: int, description: str) -> int:
        return int.from_bytes(self.take(size, description), "big")

    def tpm2b(self, description: str) -> bytes:
        size = self.integer(2, f"{description} size")
        return self.take(size, description)


def sha384(data: bytes) -> bytes:
    return hashlib.sha384(data).digest()


def pcr_extend(current: bytes, measurement: bytes) -> bytes:
    return sha384(current + sha384(measurement))


def replay_srtm(regions: Sequence[MeasurementRegion]) -> bytes:
    pcr = bytes(SHA384_SIZE)
    pmp_bytes = b"".join(value.to_bytes(8, "little") for value in PMP_STATE)
    pcr = pcr_extend(pcr, pmp_bytes)
    for region in regions:
        pcr = pcr_extend(pcr, region.read())
    return pcr


def replay_drtm(region: MeasurementRegion) -> bytes:
    return pcr_extend(bytes(SHA384_SIZE), region.read())


def rust_usize_constant(path: Path, name: str) -> int:
    try:
        source = path.read_text(encoding="utf-8")
    except OSError as error:
        raise VerificationError(f"cannot read configuration {path}: {error}") from error
    match = re.search(
        rf"\b(?:pub\s+)?const\s+{re.escape(name)}\s*:\s*usize\s*=\s*"
        rf"(0x[0-9a-fA-F_]+|[0-9][0-9_]*)\s*;",
        source,
    )
    if not match:
        raise VerificationError(f"cannot find usize constant {name} in {path}")
    return int(match.group(1).replace("_", ""), 0)


def parse_decimal_array(text: str, pattern: str, description: str) -> bytes:
    matches = list(re.finditer(pattern, text, flags=re.DOTALL | re.MULTILINE))
    if len(matches) != 1:
        raise VerificationError(
            f"expected exactly one {description} in log, found {len(matches)}"
        )
    values = [int(value) for value in re.findall(r"\d+", matches[0].group(1))]
    if not values or any(value > 255 for value in values):
        raise VerificationError(f"invalid byte array for {description}")
    return bytes(values)


def parse_log(path: Path) -> LogValues:
    try:
        text = path.read_text(encoding="utf-8", errors="replace")
    except OSError as error:
        raise VerificationError(f"cannot read run log {path}: {error}") from error

    pcr0 = parse_decimal_array(
        text,
        r"Printing PCR 0's digest after SRTM:\s*\n\s*\[([^]]+)\]",
        "final SRTM PCR 0",
    )
    pcr17_matches = list(
        re.finditer(r"Printing PCR 17's digest\s*:\s*\[([^]]+)\]", text, re.DOTALL)
    )
    if len(pcr17_matches) != 1:
        raise VerificationError(
            "expected exactly one post-DRTM PCR 17 byte array in log, "
            f"found {len(pcr17_matches)}"
        )
    pcr17_values = [int(value) for value in re.findall(r"\d+", pcr17_matches[0].group(1))]
    if not pcr17_values or any(value > 255 for value in pcr17_values):
        raise VerificationError("invalid byte array for final DRTM PCR 17")
    pcr17 = bytes(pcr17_values)

    drtm_matches = list(
        re.finditer(
            r"Starting DRTM operations with args:\s*"
            r"0x([0-9a-fA-F]+),\s*0x([0-9a-fA-F]+)",
            text,
        )
    )
    if len(drtm_matches) != 1:
        raise VerificationError(
            f"expected exactly one DRTM address/size in log, found {len(drtm_matches)}"
        )

    quote_marker = "tpm quote response:"
    if text.count(quote_marker) != 1:
        raise VerificationError(
            f"expected exactly one TPM quote response in log, found {text.count(quote_marker)}"
        )
    quote_text = text.split(quote_marker, 1)[1]
    attestation = parse_decimal_array(
        quote_text, r"\bATTESTATION:\s*\n\s*0x\[([^]]+)\]", "quote attestation"
    )
    modulus = parse_decimal_array(
        quote_text, r"\bMODULUS:\s*\n\s*0x\[([^]]+)\]", "quote modulus"
    )
    signature = parse_decimal_array(
        quote_text, r"\bSIGN:\s*\n\s*0x\[([^]]+)\]", "quote signature"
    )

    for description, value in (("PCR 0", pcr0), ("PCR 17", pcr17)):
        if len(value) != SHA384_SIZE:
            raise VerificationError(
                f"logged {description} is {len(value)} bytes, expected {SHA384_SIZE}"
            )
    for description, value in (("modulus", modulus), ("signature", signature)):
        if len(value) != RSA_KEY_SIZE:
            raise VerificationError(
                f"logged quote {description} is {len(value)} bytes, expected {RSA_KEY_SIZE}"
            )

    return LogValues(
        pcr0=pcr0,
        pcr17=pcr17,
        drtm_address=int(drtm_matches[0].group(1), 16),
        drtm_size=int(drtm_matches[0].group(2), 16),
        attestation_buffer=attestation,
        modulus=modulus,
        signature=signature,
    )


def parse_attestation(buffer: bytes) -> Attestation:
    reader = BigEndianReader(buffer)
    magic = reader.integer(4, "magic")
    attest_type = reader.integer(2, "type")
    qualified_signer = reader.tpm2b("qualified signer")
    extra_data = reader.tpm2b("extra data")

    reader.take(8, "clock")
    reader.take(4, "reset count")
    reader.take(4, "restart count")
    reader.take(1, "safe flag")
    reader.take(8, "firmware version")

    selection_count = reader.integer(4, "PCR selection count")
    if selection_count > 16:
        raise VerificationError(f"unreasonable PCR selection count {selection_count}")
    selections: list[PcrSelection] = []
    for index in range(selection_count):
        algorithm = reader.integer(2, f"PCR selection {index} algorithm")
        bitmap_size = reader.integer(1, f"PCR selection {index} bitmap size")
        bitmap = reader.take(bitmap_size, f"PCR selection {index} bitmap")
        selections.append(PcrSelection(algorithm, bitmap))
    pcr_digest = reader.tpm2b("quoted PCR digest")

    raw = buffer[: reader.offset]
    padding = buffer[reader.offset :]
    if any(padding):
        raise VerificationError("non-zero bytes follow the parsed TPM attestation")
    return Attestation(
        raw=raw,
        magic=magic,
        attest_type=attest_type,
        qualified_signer=qualified_signer,
        extra_data=extra_data,
        selections=tuple(selections),
        pcr_digest=pcr_digest,
    )


def verify_rsassa_sha384(message: bytes, modulus: bytes, signature: bytes) -> bool:
    """Verify an RSA PKCS#1 v1.5 SHA-384 signature without dependencies."""
    modulus_integer = int.from_bytes(modulus, "big")
    signature_integer = int.from_bytes(signature, "big")
    if modulus_integer <= 0 or signature_integer >= modulus_integer:
        return False

    encoded = pow(signature_integer, 65537, modulus_integer).to_bytes(len(modulus), "big")
    # ASN.1 DigestInfo prefix for SHA-384.
    digest_info = bytes.fromhex("3041300d060960864801650304020205000430") + sha384(message)
    padding_size = len(modulus) - len(digest_info) - 3
    if padding_size < 8:
        return False
    expected = b"\x00\x01" + b"\xff" * padding_size + b"\x00" + digest_info
    return hmac.compare_digest(encoded, expected)


def quoted_pcr_digest(pcr0: bytes, pcr17: bytes) -> bytes:
    return sha384(pcr0 + pcr17)


def format_digest(value: bytes) -> str:
    return value.hex()


def print_comparison(name: str, computed: bytes, reported: bytes) -> bool:
    passed = hmac.compare_digest(computed, reported)
    print(f"[{'PASS' if passed else 'FAIL'}] {name}")
    print(f"       computed: {format_digest(computed)}")
    print(f"       reported: {format_digest(reported)}")
    return passed


def build_argument_parser(repository: Path) -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--log",
        type=Path,
        default=repository / "expected_results/flashpoint-drtm-qemu-run-log.txt",
        help="QEMU run log containing PCRs and quote (default: %(default)s)",
    )
    parser.add_argument(
        "--anchor",
        type=Path,
        default=repository / "target/riscv-unknown-kernel/debug/anchor",
        help="anchor ELF (default: %(default)s)",
    )
    parser.add_argument(
        "--tpm-driver",
        type=Path,
        default=repository / "target/riscv-unknown-kernel/debug/tpm-driver",
        help="TPM-driver ELF (default: %(default)s)",
    )
    parser.add_argument(
        "--tyche",
        type=Path,
        default=repository / "tyche/target/riscv-unknown-kernel/release/tyche",
        help="Tyche ELF (default: %(default)s)",
    )
    return parser


def measurement_regions(
    repository: Path, anchor: ElfImage, tpm_driver: ElfImage
) -> tuple[MeasurementRegion, ...]:
    config = repository / "tpm-driver/src/msmt_cfg.rs"
    definitions = (
        ("anchor entry point + text", anchor, "ANCHOR_ENTRY_TEXT"),
        ("anchor rodata", anchor, "ANCHOR_RODATA"),
        ("anchor GOT", anchor, "ANCHOR_GOT"),
        ("TPM-driver entry point + text", tpm_driver, "TPM_DRV_ENTRY_TEXT"),
        ("TPM-driver rodata", tpm_driver, "TPM_DRV_RODATA"),
        ("TPM-driver GOT", tpm_driver, "TPM_DRV_GOT"),
    )
    return tuple(
        MeasurementRegion(
            name=name,
            image=image,
            address=rust_usize_constant(config, f"{prefix}_ADDRESS"),
            size=rust_usize_constant(config, f"{prefix}_SIZE"),
        )
        for name, image, prefix in definitions
    )


def verify_quote(
    values: LogValues, computed_pcr0: bytes, computed_pcr17: bytes
) -> tuple[bool, list[tuple[str, bool]]]:
    attestation = parse_attestation(values.attestation_buffer)
    expected_selection = (
        len(attestation.selections) == 1
        and attestation.selections[0].algorithm == TPM_ALG_SHA384
        and attestation.selections[0].indices == (PCR0, PCR17)
    )
    expected_format = (
        attestation.magic == TPM_GENERATED_VALUE
        and attestation.attest_type == TPM_ST_ATTEST_QUOTE
        and len(attestation.pcr_digest) == SHA384_SIZE
        and len(attestation.qualified_signer) > 0
    )
    signature_valid = verify_rsassa_sha384(
        attestation.raw, values.modulus, values.signature
    )
    reported_pcrs_bound = hmac.compare_digest(
        attestation.pcr_digest, quoted_pcr_digest(values.pcr0, values.pcr17)
    )
    computed_pcrs_bound = hmac.compare_digest(
        attestation.pcr_digest, quoted_pcr_digest(computed_pcr0, computed_pcr17)
    )
    details = [
        ("TPMS_ATTEST magic/type and fields", expected_format),
        ("quote selects SHA-384 PCRs 0 and 17", expected_selection),
        ("RSA-3072 RSASSA/SHA-384 signature", signature_valid),
        ("attested digest matches reported PCRs", reported_pcrs_bound),
        ("attested digest matches recomputed PCRs", computed_pcrs_bound),
    ]
    return all(passed for _, passed in details), details


def main(argv: Sequence[str] | None = None) -> int:
    repository = Path(__file__).resolve().parent.parent
    args = build_argument_parser(repository).parse_args(argv)

    try:
        values = parse_log(args.log.resolve())
        anchor = ElfImage(args.anchor.resolve())
        tpm_driver = ElfImage(args.tpm_driver.resolve())
        tyche = ElfImage(args.tyche.resolve())

        srtm_regions = measurement_regions(repository, anchor, tpm_driver)
        computed_pcr0 = replay_srtm(srtm_regions)

        anchor_source = repository / "anchor/src/main.rs"
        drtm_address = rust_usize_constant(anchor_source, "SECURITY_MONITOR_ADDR")
        drtm_size = rust_usize_constant(anchor_source, "TRUSTED_MEASUREMENT_SIZE")
        drtm_region = MeasurementRegion("Tyche security monitor", tyche, drtm_address, drtm_size)
        computed_pcr17 = replay_drtm(drtm_region)

        print("FlashPoint measurement and quote verification")
        print(f"  log:        {args.log.resolve()}")
        print(f"  anchor:     {args.anchor.resolve()}")
        print(f"  TPM driver: {args.tpm_driver.resolve()}")
        print(f"  Tyche:      {args.tyche.resolve()}")
        print()

        srtm_ok = print_comparison("SRTM measurement (PCR 0)", computed_pcr0, values.pcr0)
        print("       replayed 7 extends: PMP state, then 6 configured binary regions")
        for region in srtm_regions:
            print(
                f"       {region.name}: 0x{region.address:x} + 0x{region.size:x}"
            )
        print()

        drtm_parameters_ok = (
            values.drtm_address == drtm_address and values.drtm_size == drtm_size
        )
        drtm_digest_ok = print_comparison(
            "DRTM measurement (PCR 17)", computed_pcr17, values.pcr17
        )
        print(
            f"       configured: 0x{drtm_address:x} + 0x{drtm_size:x}; "
            f"logged: 0x{values.drtm_address:x} + 0x{values.drtm_size:x}"
        )
        if not drtm_parameters_ok:
            print("       FAIL: logged DRTM address/size differs from anchor configuration")
        drtm_ok = drtm_digest_ok and drtm_parameters_ok
        print()

        quote_ok, quote_details = verify_quote(values, computed_pcr0, computed_pcr17)
        print(f"[{'PASS' if quote_ok else 'FAIL'}] TPM quote")
        for description, passed in quote_details:
            print(f"       {'PASS' if passed else 'FAIL'}: {description}")
        print(
            "       note: the log supplies the public key and contains no nonce; "
            "key trust and freshness are not established"
        )
        print()

        overall = srtm_ok and drtm_ok and quote_ok
        print(f"Overall result: {'PASS' if overall else 'FAIL'}")
        return 0 if overall else 1
    except VerificationError as error:
        print(f"Verification error: {error}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
