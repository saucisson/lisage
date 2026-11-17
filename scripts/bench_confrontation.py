"""Confrontation benchmark orchestrator (M7-5, issue #115, ADR-0102 protocol).

Runs the M7-4 real charges against each witness — the lisage
``confrontation`` binary and the OxiDD container — with **one fresh
process per measurement** (the peak RSS is a whole-process measurement,
it does not divide into in-process repetitions). Raw JSONL lines are
deposited in ``bench-results/<run-id>/<charge>-<outil>.jsonl`` and the
aggregates (median, min, max) in ``bench-results/<run-id>/aggregats.json``.

Fairness (protocol): same input, same object built, same cross-check
value. The witnesses receive the exact charge serialized by lisage
itself (``confrontation dump-charge``), and ``noeuds``/``verifie`` must
be identical across tools for the same charge — any divergence
invalidates the campaign (exit 1).

Local only: never run in CI (ADR-0102), Docker required for the OxiDD
witness.
"""

import json
import statistics
import subprocess
import sys
from dataclasses import dataclass
from datetime import UTC, datetime
from pathlib import Path
from typing import TYPE_CHECKING, cast

from scripts._shared import (
    EXIT_CODE_CRITERIA_UNMET,
    EXIT_CODE_PROTOCOL_FAILURE,
    EXIT_CODE_SUCCESS,
)

if TYPE_CHECKING:
    import argparse
    from collections.abc import Sequence

DEFAULT_REPETITIONS = 5
DEFAULT_OUTPUT_DIRECTORY = Path("bench-results")
DEFAULT_BINARY = Path("target/release/confrontation")
DEFAULT_OXIDD_IMAGE = "lisage-oxidd:latest"
DEFAULT_TOOLS = ("lisage", "oxidd")

MEASURE_KEYS = (
    "outil",
    "variante",
    "charge",
    "parametres",
    "temps_ms",
    "rss_max_ko",
    "noeuds",
    "verifie",
)


@dataclass(frozen=True)
class Charge:
    """One real benchmark charge (M7-4): label, workload, versioned input."""

    label: str
    workload: str
    path: Path


CHARGES: tuple[Charge, ...] = (
    Charge("cnf-uf20-01", "cnf", Path("crates/lisage-bench/models/uf20-01.cnf")),
    Charge("pnml-swimmingpool-1", "pnml", Path("crates/lisage-bench/models/SwimmingPool-1.pnml")),
    Charge("pnml-erk-000001", "pnml", Path("crates/lisage-bench/models/erk-000001.pnml")),
)


def parse_measure_line(*, line: str) -> dict[str, object]:
    """Parse one JSONL measurement line and validate the protocol keys."""
    measure = json.loads(line)
    if not isinstance(measure, dict):
        raise ValueError(f"measurement line is not an object: {line!r}")
    missing = [key for key in MEASURE_KEYS if key not in measure]
    if missing:
        raise ValueError(f"measurement line misses keys {missing}: {line!r}")
    return measure


def aggregate_measures(*, measures: Sequence[dict[str, object]]) -> dict[str, object]:
    """Aggregate one charge x one tool.

    Median/min/max of the time and memory, plus the constant cross-check
    values (``noeuds``, ``verifie``).
    """
    if not measures:
        raise ValueError("no measurement to aggregate")
    for key in ("noeuds", "verifie"):
        values = {measure[key] for measure in measures}
        if len(values) != 1:
            raise ValueError(f"non-constant {key} across repetitions: {values}")
    temps = [float(cast("float", measure["temps_ms"])) for measure in measures]
    rss = [int(cast("int", measure["rss_max_ko"])) for measure in measures]
    return {
        "outil": measures[0]["outil"],
        "charge": measures[0]["charge"],
        "repetitions": len(measures),
        "temps_ms": {
            "mediane": statistics.median(temps),
            "min": min(temps),
            "max": max(temps),
        },
        "rss_max_ko": {
            "mediane": statistics.median(rss),
            "min": min(rss),
            "max": max(rss),
        },
        "noeuds": measures[0]["noeuds"],
        "verifie": measures[0]["verifie"],
    }


def cross_check_aggregates(*, aggregates: Sequence[dict[str, object]]) -> list[str]:
    """Cross-tool check: for one charge, ``noeuds`` and ``verifie`` must be identical across tools.

    Returns the list of divergences.
    """
    by_charge: dict[str, list[dict[str, object]]] = {}
    for aggregate in aggregates:
        by_charge.setdefault(str(aggregate["charge"]), []).append(aggregate)
    divergences: list[str] = []
    for charge, group in sorted(by_charge.items()):
        for key in ("noeuds", "verifie"):
            values = {(str(aggregate["outil"]), aggregate[key]) for aggregate in group}
            distinct = {value for _, value in values}
            if len(distinct) != 1:
                divergences.append(
                    f"charge {charge}: {key} diverges across tools: {sorted(values)}"
                )
    return divergences


def run_lisage_measurement(*, binary: Path, charge: Charge) -> dict[str, object]:
    """One lisage measurement: a fresh process running the workload once."""
    completed = subprocess.run(
        [str(binary), charge.workload, str(charge.path)],
        capture_output=True,
        text=True,
        check=False,
    )
    if completed.returncode != EXIT_CODE_SUCCESS:
        raise RuntimeError(f"lisage measurement failed: {completed.stderr.strip()}")
    return parse_measure_line(line=completed.stdout.strip())


def dump_charge(*, binary: Path, charge: Charge) -> str:
    """Serialize the charge in the standard stdin protocol of the witnesses.

    The witnesses receive the exact input lisage measured.
    """
    completed = subprocess.run(
        [str(binary), "dump-charge", charge.workload, str(charge.path)],
        capture_output=True,
        text=True,
        check=False,
    )
    if completed.returncode != EXIT_CODE_SUCCESS:
        raise RuntimeError(f"dump-charge failed: {completed.stderr.strip()}")
    return completed.stdout


def run_oxidd_measurement(*, image: str, charge_stdin: str, label: str) -> dict[str, object]:
    """One OxiDD measurement: a fresh container, charge on stdin."""
    completed = subprocess.run(
        ["docker", "run", "--rm", "-i", image, "oxidd_count", label],
        input=charge_stdin,
        capture_output=True,
        text=True,
        check=False,
    )
    if completed.returncode != EXIT_CODE_SUCCESS:
        raise RuntimeError(f"oxidd measurement failed: {completed.stderr.strip()}")
    return parse_measure_line(line=completed.stdout.strip())


def run_campaign(
    *,
    charges: Sequence[Charge],
    tools: Sequence[str],
    repetitions: int,
    binary: Path,
    image: str,
    output_directory: Path,
    run_identifier: str,
) -> int:
    """Run the whole campaign and deposit raw lines + aggregates."""
    run_directory = output_directory / run_identifier
    run_directory.mkdir(parents=True, exist_ok=True)
    aggregates: list[dict[str, object]] = []
    for charge in charges:
        charge_stdin: str | None = None
        for tool in tools:
            measures: list[dict[str, object]] = []
            for _ in range(repetitions):
                if tool == "lisage":
                    measure = run_lisage_measurement(binary=binary, charge=charge)
                elif tool == "oxidd":
                    if charge_stdin is None:
                        charge_stdin = dump_charge(binary=binary, charge=charge)
                    measure = run_oxidd_measurement(
                        image=image, charge_stdin=charge_stdin, label=charge.label
                    )
                else:
                    raise ValueError(f"unknown tool: {tool}")
                # The binary reports the workload as `charge`; the campaign
                # identifies the charge by its label, uniformly across tools.
                measure["charge"] = charge.label
                measures.append(measure)
                line = json.dumps(measure, sort_keys=True)
                print(f"[{run_identifier}] {charge.label} / {tool}: {line}")
            raw_path = run_directory / f"{charge.label}-{tool}.jsonl"
            raw_path.write_text(
                "".join(json.dumps(measure, sort_keys=True) + "\n" for measure in measures),
                encoding="utf-8",
            )
            aggregates.append(aggregate_measures(measures=measures))
    divergences = cross_check_aggregates(aggregates=aggregates)
    aggregates_path = run_directory / "aggregats.json"
    aggregates_path.write_text(
        json.dumps(
            {"run": run_identifier, "aggregats": aggregates, "divergences": divergences},
            indent=2,
        )
        + "\n",
        encoding="utf-8",
    )
    print(f"[{run_identifier}] raw lines + aggregates deposited in {run_directory}")
    if divergences:
        for divergence in divergences:
            print(f"DIVERGENCE: {divergence}", file=sys.stderr)
        return EXIT_CODE_CRITERIA_UNMET
    return EXIT_CODE_SUCCESS


def register(subparsers: argparse._SubParsersAction[argparse.ArgumentParser]) -> None:
    """Register the bench-confrontation subcommand."""
    subparser = subparsers.add_parser(
        "bench-confrontation",
        help="Run the confrontation benchmark campaign (local, ADR-0102; never in CI).",
        description=(
            "Run the M7-4 charges against lisage and the OxiDD witness, "
            "one fresh process per measurement (M7-5)."
        ),
    )
    subparser.add_argument(
        "--repetitions",
        type=int,
        default=DEFAULT_REPETITIONS,
        help="Measurements per charge x tool.",
    )
    subparser.add_argument(
        "--tools",
        type=str,
        default=",".join(DEFAULT_TOOLS),
        help="Comma-separated tools: lisage,oxidd.",
    )
    subparser.add_argument(
        "--charges",
        type=str,
        default="",
        help="Comma-separated charge labels (default: all).",
    )
    subparser.add_argument(
        "--binary",
        type=Path,
        default=DEFAULT_BINARY,
        help="Path of the release confrontation binary.",
    )
    subparser.add_argument(
        "--image",
        type=str,
        default=DEFAULT_OXIDD_IMAGE,
        help="OxiDD witness image.",
    )
    subparser.add_argument(
        "--output-dir",
        type=Path,
        default=DEFAULT_OUTPUT_DIRECTORY,
        help="Directory of the deposited results.",
    )
    subparser.add_argument(
        "--run-id",
        type=str,
        default="",
        help="Run identifier (default: UTC timestamp).",
    )
    subparser.set_defaults(runner=run)


def run(*, arguments: argparse.Namespace) -> int:
    """Entry point: run the campaign, deposit results, report divergences."""
    labels = [label.strip() for label in arguments.charges.split(",") if label.strip()]
    charges = [charge for charge in CHARGES if not labels or charge.label in labels]
    if not charges:
        print("Error: no charge selected", file=sys.stderr)
        return EXIT_CODE_PROTOCOL_FAILURE
    for charge in charges:
        if not charge.path.is_file():
            print(f"Error: charge input missing: {charge.path}", file=sys.stderr)
            return EXIT_CODE_PROTOCOL_FAILURE
    tools = [tool.strip() for tool in arguments.tools.split(",") if tool.strip()]
    run_identifier = arguments.run_id or datetime.now(UTC).strftime("%Y%m%dT%H%M%SZ")
    try:
        return run_campaign(
            charges=charges,
            tools=tools,
            repetitions=arguments.repetitions,
            binary=arguments.binary,
            image=arguments.image,
            output_directory=arguments.output_dir,
            run_identifier=run_identifier,
        )
    except RuntimeError as error:
        # A measurement failed: the campaign criteria are unmet (exit 1).
        print(f"Error: {error}", file=sys.stderr)
        return EXIT_CODE_CRITERIA_UNMET
    except ValueError as error:
        # Usage/config error: protocol failure (exit 2).
        print(f"Error: {error}", file=sys.stderr)
        return EXIT_CODE_PROTOCOL_FAILURE
