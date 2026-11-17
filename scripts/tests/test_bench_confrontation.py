"""Tests of the confrontation benchmark orchestrator (M7-5, issue #115).

The campaign itself (processes, Docker) is not run here: the pure parts
are pinned — JSONL parsing, aggregation (median/min/max), the cross-tool
divergence check, and the integrity of the charge list.
"""

import json

import pytest

from scripts.bench_confrontation import (
    CHARGES,
    aggregate_measures,
    cross_check_aggregates,
    parse_measure_line,
)


def make_measure(
    *,
    outil: str,
    charge: str,
    temps_ms: float,
    rss: int,
    noeuds: int = 51,
    verifie: int = 8,
) -> dict[str, object]:
    """One protocol-conformant measurement line."""
    return {
        "outil": outil,
        "variante": "bdd",
        "charge": charge,
        "parametres": {"n": "20"},
        "temps_ms": temps_ms,
        "rss_max_ko": rss,
        "noeuds": noeuds,
        "verifie": verifie,
    }


def test_parse_measure_line_pins_protocol_keys() -> None:
    """A conformant line parses; a line missing a key is rejected."""
    line = json.dumps(make_measure(outil="lisage", charge="cnf-uf20-01", temps_ms=1.5, rss=4096))
    measure = parse_measure_line(line=line)
    assert measure["outil"] == "lisage"
    assert measure["noeuds"] == 51
    broken = json.dumps({"outil": "lisage", "charge": "x"})
    with pytest.raises(ValueError, match="misses keys"):
        parse_measure_line(line=broken)


def test_aggregate_mediane_min_max() -> None:
    """Median/min/max of time and RSS over the repetitions; noeuds/verifie constant."""
    measures = [
        make_measure(outil="lisage", charge="c", temps_ms=10.0, rss=100),
        make_measure(outil="lisage", charge="c", temps_ms=20.0, rss=200),
        make_measure(outil="lisage", charge="c", temps_ms=30.0, rss=300),
    ]
    aggregate = aggregate_measures(measures=measures)
    assert aggregate["repetitions"] == 3
    assert aggregate["temps_ms"] == {"mediane": 20.0, "min": 10.0, "max": 30.0}
    assert aggregate["rss_max_ko"] == {"mediane": 200, "min": 100, "max": 300}
    assert aggregate["noeuds"] == 51
    assert aggregate["verifie"] == 8


def test_aggregate_rejecte_verifie_non_constant() -> None:
    """A cross-check value that varies across repetitions invalidates the run."""
    measures = [
        make_measure(outil="lisage", charge="c", temps_ms=1.0, rss=1, verifie=8),
        make_measure(outil="lisage", charge="c", temps_ms=2.0, rss=2, verifie=9),
    ]
    with pytest.raises(ValueError, match="non-constant verifie"):
        aggregate_measures(measures=measures)


def test_cross_check_detecte_divergence_entre_outils() -> None:
    """Same charge, two tools, different node counts: divergence reported."""
    aggregates = [
        aggregate_measures(
            measures=[make_measure(outil="lisage", charge="c", temps_ms=1.0, rss=1, noeuds=51)]
        ),
        aggregate_measures(
            measures=[make_measure(outil="oxidd", charge="c", temps_ms=2.0, rss=2, noeuds=49)]
        ),
    ]
    divergences = cross_check_aggregates(aggregates=aggregates)
    assert len(divergences) == 1
    assert "noeuds" in divergences[0]
    # Identical values across tools: no divergence.
    aggregates_ok = [
        aggregate_measures(
            measures=[make_measure(outil="lisage", charge="c", temps_ms=1.0, rss=1, noeuds=51)]
        ),
        aggregate_measures(
            measures=[make_measure(outil="oxidd", charge="c", temps_ms=2.0, rss=2, noeuds=51)]
        ),
    ]
    assert cross_check_aggregates(aggregates=aggregates_ok) == []


def test_charges_versionnees_existent() -> None:
    """Every declared charge points at a versioned input file of the repo."""
    assert len(CHARGES) >= 3
    for charge in CHARGES:
        assert charge.path.is_file(), f"missing charge input: {charge.path}"
        assert charge.workload in ("cnf", "pnml")
