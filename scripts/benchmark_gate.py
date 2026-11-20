#!/usr/bin/env python3
"""Benchmark gate (ADR-0114): Actus kernels against their C twins.

Each kernel in `benchmarks/kernels` is built twice: the Actus source with
the `release` profile on the LLVM backend, and the C source with
`clang -O3`. The Actus kernel uses wrapping arithmetic (`+%`), the
arithmetic of its C twin, so the comparison measures what Actus hands LLVM.
`<kernel>_checked.act` is the same kernel with trapping arithmetic
(ADR-0115); its cost over the wrapping kernel is reported. All programs run
under Callgrind, which counts executed instructions deterministically,
collecting from `main`. The gate fails when an Actus kernel exits
differently from C, when the wrapping kernel executes more than `threshold`
times the instructions of C, or when either Actus kernel executes more than
`ratchet_tolerance` above its recorded baseline.
"""

from __future__ import annotations

import argparse
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tempfile
import tomllib


def arguments() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--actus", type=Path, required=True, help="actus compiler binary")
    parser.add_argument("--cc", default=os.environ.get("CC_GATE", "clang"), help="C compiler")
    parser.add_argument(
        "--kernels",
        type=Path,
        default=Path(__file__).resolve().parent.parent / "benchmarks/kernels",
    )
    parser.add_argument(
        "--update-baseline",
        action="store_true",
        help="write the measured Actus counts as the new baseline",
    )
    return parser.parse_args()


def run(command: list[str]) -> subprocess.CompletedProcess[str]:
    return subprocess.run(command, capture_output=True, text=True, check=False)


def build_actus(actus: Path, source: Path, output: Path) -> None:
    result = run(
        [str(actus), "build", str(source), "--profile", "release", "--backend", "llvm"]
        + ["--emit", "exe", "-o", str(output)]
    )
    if result.returncode != 0:
        raise SystemExit(f"actus build of {source.name} failed:\n{result.stderr}")


def build_c(cc: str, source: Path, output: Path) -> None:
    result = run([cc, "-O3", str(source), "-o", str(output)])
    if result.returncode != 0:
        raise SystemExit(f"{cc} build of {source.name} failed:\n{result.stderr}")


def measure(program: Path) -> tuple[int, int]:
    """The exit status of `program` and the instructions it executed from `main`."""
    result = run(
        ["valgrind", "--tool=callgrind", "--toggle-collect=main", "--callgrind-out-file=/dev/null"]
        + [str(program)]
    )
    collected = re.search(r"Collected : (\d+)", result.stderr)
    if collected is None:
        raise SystemExit(f"callgrind reported no instruction count for {program.name}:\n{result.stderr}")
    return result.returncode, int(collected.group(1))


def write_baseline(path: Path, counts: dict[str, int]) -> None:
    """Rewrites the `[baseline]` table, which closes the file."""
    text = path.read_text(encoding="utf-8")
    head = text[: text.index("[baseline]")]
    table = "".join(f"{name} = {count}\n" for name, count in sorted(counts.items()))
    path.write_text(f"{head}[baseline]\n{table}", encoding="utf-8")


def measure_kernel(args: argparse.Namespace, directory: Path, name: str) -> dict[str, int]:
    """Builds and measures the wrapping and checked Actus kernels and the C twin."""
    counts = {}
    for label, source, build in (
        ("actus", args.kernels / f"{name}.act", lambda s, o: build_actus(args.actus.resolve(), s, o)),
        ("checked", args.kernels / f"{name}_checked.act", lambda s, o: build_actus(args.actus.resolve(), s, o)),
        ("c", args.kernels / f"{name}.c", lambda s, o: build_c(args.cc, s, o)),
    ):
        program = directory / f"{name}-{label}"
        build(source, program)
        counts[f"{label}_status"], counts[label] = measure(program)
    return counts


def kernel_failures(name: str, counts: dict[str, int], config: dict, update: bool) -> list[str]:
    threshold = float(config["threshold"])
    tolerance = float(config["ratchet_tolerance"])
    failures = []
    for label in ("actus", "checked"):
        if counts[f"{label}_status"] != counts["c_status"]:
            failures.append(
                f"{name} ({label}): Actus exited {counts[f'{label}_status']}, C exited {counts['c_status']}"
            )
    ratio = counts["actus"] / counts["c"]
    if ratio > threshold:
        failures.append(f"{name}: {ratio:.3f}x the instructions of C exceeds {threshold}x")
    for key, label in ((name, "actus"), (f"{name}_checked", "checked")):
        if update:
            continue
        if key not in config["baseline"]:
            failures.append(f"{key}: no baseline; record one with --update-baseline")
            continue
        change = counts[label] / config["baseline"][key] - 1
        if change > tolerance:
            failures.append(f"{key}: {change:+.1%} against its baseline exceeds {tolerance:+.0%}")
    return failures


def report(rows: list[tuple[str, dict[str, int]]], config: dict) -> str:
    baseline = config["baseline"]
    lines = [
        f"## Benchmark gate (threshold {config['threshold']}x C `-O3`)",
        "",
        "Wrapping kernels match the arithmetic of their C twins; checked kernels",
        "use trapping arithmetic (ADR-0115) and show the cost of overflow checks.",
        "",
        "| kernel | Actus | C | ratio | baseline | checked | check cost | baseline | exit |",
        "|---|---:|---:|---:|---:|---:|---:|---:|---:|",
    ]
    for name, counts in rows:
        lines.append(
            f"| {name} | {counts['actus']:,} | {counts['c']:,} | {counts['actus'] / counts['c']:.3f} "
            f"| {counts['actus'] / baseline.get(name, counts['actus']) - 1:+.1%} "
            f"| {counts['checked']:,} | {counts['checked'] / counts['actus'] - 1:+.1%} "
            f"| {counts['checked'] / baseline.get(f'{name}_checked', counts['checked']) - 1:+.1%} "
            f"| {counts['c_status']} |"
        )
    return "\n".join(lines) + "\n"


def main() -> int:
    args = arguments()
    for tool in ("valgrind", args.cc):
        if shutil.which(tool) is None:
            raise SystemExit(f"benchmark gate requires `{tool}` on PATH")
    config = tomllib.loads((args.kernels / "gate.toml").read_text(encoding="utf-8"))
    rows = []
    failures = []
    with tempfile.TemporaryDirectory(prefix="actus-benchmark-gate-") as temporary:
        for name in config["kernels"]:
            counts = measure_kernel(args, Path(temporary), name)
            rows.append((name, counts))
            failures += kernel_failures(name, counts, config, args.update_baseline)
    text = report(rows, config)
    print(text)
    summary = os.environ.get("GITHUB_STEP_SUMMARY")
    if summary:
        with open(summary, "a", encoding="utf-8") as handle:
            handle.write(text)
    if args.update_baseline:
        counts = {name: row["actus"] for name, row in rows}
        counts.update({f"{name}_checked": row["checked"] for name, row in rows})
        write_baseline(args.kernels / "gate.toml", counts)
    for failure in failures:
        print(f"FAIL {failure}", file=sys.stderr)
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
