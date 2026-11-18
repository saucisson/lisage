"""Tests of the confrontation benchmark orchestrator (M7-5, issue #115).

The campaign itself (processes, Docker) is not run here: the pure parts
are pinned — JSONL parsing, aggregation (median/min/max), the cross-tool
divergence check, and the integrity of the charge list.
"""

import json
from pathlib import Path

import pytest

from scripts import bench_confrontation as bc
from scripts.bench_confrontation import (
    CHARGES,
    PILOT_CHARGES,
    aggregate_measures,
    charge_parameters,
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
        assert charge.path is not None and charge.path.is_file(), (
            f"missing charge input: {charge.path}"
        )
        assert charge.workload in ("cnf", "pnml")


def test_pilot_charges_mem7() -> None:
    """MEM-7 pilot charges (issue #123): the three pilot workloads, with
    their parameters and their witness restriction."""
    by_label = {charge.label: charge for charge in PILOT_CHARGES}
    assert set(by_label) == {
        "alea-lcg-20",
        "philosophers-n5",
        "window-4x10-absolu",
        "window-4x10-relatif",
    }
    # alea-lcg: deterministic LCG charge, cross-checked against OxiDD.
    alea = by_label["alea-lcg-20"]
    assert alea.workload == "alea-lcg"
    assert charge_parameters(charge=alea) == ("20",)
    assert "oxidd" in alea.tools
    # philosophers: versioned states file, lisage-only (no DDD witness).
    philo = by_label["philosophers-n5"]
    assert philo.workload == "philosophers"
    assert philo.path is not None and philo.path.is_file()
    assert charge_parameters(charge=philo) == ("5", str(philo.path))
    assert philo.tools == ("lisage",)
    # window: parameterized (size=4, nb_bases=10 — the pinned values of
    # tests/confrontation.rs), both encodings, lisage-only.
    for label, encodage in (("window-4x10-absolu", "absolu"), ("window-4x10-relatif", "relatif")):
        window = by_label[label]
        assert window.workload == "window"
        assert charge_parameters(charge=window) == ("4", "10", encodage)
        assert window.tools == ("lisage",)


def test_charge_parameters_default_path() -> None:
    """A file-based charge without explicit parameters runs on its path."""
    charge = bc.Charge("c", "cnf", Path("x"))
    assert charge_parameters(charge=charge) == ("x",)
    charge_sans_fichier = bc.Charge("c", "alea-lcg", parameters=("12",))
    assert charge_parameters(charge=charge_sans_fichier) == ("12",)


def test_run_campaign_exit_1_sur_divergence(
    monkeypatch: pytest.MonkeyPatch, tmp_path: Path
) -> None:
    """Acceptance criterion: a cross-tool divergence on noeuds/verifie exits 1.

    The raw lines and the aggregates (with the divergence recorded) are
    still deposited as evidence.
    """
    charge = bc.Charge("c", "cnf", Path("x"))

    def lisage_measure(*, binary: Path, charge: bc.Charge) -> dict[str, object]:
        return make_measure(outil="lisage", charge="c", temps_ms=1.0, rss=1, noeuds=51)

    def oxidd_measure(*, image: str, charge_stdin: str, label: str) -> dict[str, object]:
        return make_measure(outil="oxidd", charge="c", temps_ms=2.0, rss=2, noeuds=49)

    monkeypatch.setattr(bc, "run_lisage_measurement", lisage_measure)
    monkeypatch.setattr(bc, "run_oxidd_measurement", oxidd_measure)
    monkeypatch.setattr(bc, "dump_charge", lambda *, binary, charge: "2\n0 1 1 0\n")
    exit_code = bc.run_campaign(
        charges=[charge],
        tools=["lisage", "oxidd"],
        repetitions=2,
        binary=Path("b"),
        image="img",
        output_directory=tmp_path,
        run_identifier="t",
    )
    assert exit_code == 1
    deposited = json.loads((tmp_path / "t" / "aggregats.json").read_text(encoding="utf-8"))
    assert len(deposited["divergences"]) == 1
    assert (tmp_path / "t" / "c-lisage.jsonl").is_file()
    assert (tmp_path / "t" / "c-oxidd.jsonl").is_file()
