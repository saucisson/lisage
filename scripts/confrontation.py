"""Confrontation bench: lisage against external decision-diagram witnesses.

``make bench-confrontation`` runs the time/memory harness of
docs/bench-confrontation-temoins-et-mesures.md: lisage and the external
witnesses (CUDD today; Sylvan, libDDD and Meddly land in follow-up
pull requests) each build the SAME decision diagram for the SAME
charge, in a FRESH process per repetition, and print ONE JSON line
(the protocol pinned in ``crates/lisage-bench/src/confrontation.rs``).

This driver is honest glue only. It does three things no single runner
can do: it generates the SHARED inputs (the extended-state files of
the philosophers charge, checked against the fil rouge pins at
generation time), it runs every ``(tool, charge, repetition)`` case as
a fresh process so the peak RSS is clean, and it cross-checks the
``verifie`` values - all tools must agree per charge, else the bench is
wrong somewhere and the driver says so instead of reporting numbers.

Stdlib only, on purpose: the bench must run with nothing but the
project environment (ADR-0021). Timing comes from the runners
themselves (the measured block is inside each tool, library init
excluded); the driver only aggregates what the runners measured.

The witnesses run in local Docker containers (ADR-0102, human
decision of 2026-11-10): out of CI, built and run on the human's
machine, images pinned in ``docker/``. A missing image or a missing
``docker`` binary is a protocol failure - the driver never falls back
to comparing against something else.

Outputs land in ``.confrontation/<run>/`` (git-ignored): the raw JSON
lines in ``mesures.jsonl``, the aggregated Markdown report in
``rapport.md``, and the shared state files in ``scratch/``. Stdout
carries the short summary - one line per case measured - per the
two-channel convention (ADR-0017).
"""

import dataclasses
import datetime
import itertools
import json
import statistics
import subprocess
import sys
from typing import TYPE_CHECKING

from scripts._shared import (
    EXIT_CODE_PROTOCOL_FAILURE,
    EXIT_CODE_SUCCESS,
    REPOSITORY_ROOT,
)

if TYPE_CHECKING:
    import argparse
    from collections.abc import Callable, Sequence
    from pathlib import Path

SUBCOMMAND_NAME = "bench-confrontation"
RUN_DIRECTORY_NAME = ".confrontation"
RUN_TIMESTAMP_FORMAT = "%Y%m%d-%H%M%S"
SCRATCH_DIRECTORY_NAME = "scratch"
RAW_LINES_FILE_NAME = "mesures.jsonl"
REPORT_FILE_NAME = "rapport.md"
DOCKER_SCRATCH_MOUNT = "/confrontation"

DEFAULT_REPETITIONS = 5
RUN_TIMEOUT_SECONDS = 1800

LISAGE_TOOL_NAME = "lisage"
CUDD_TOOL_NAME = "cudd"
DEFAULT_TOOLS = (LISAGE_TOOL_NAME, CUDD_TOOL_NAME)
TOOL_ORDER = (LISAGE_TOOL_NAME, CUDD_TOOL_NAME)
WITNESS_IMAGES = {CUDD_TOOL_NAME: "lisage-cudd:latest"}
WITNESS_BINARIES = {CUDD_TOOL_NAME: "cudd_bench"}

ALEA_LCG_CHARGE = "alea-lcg"
PHILOSOPHES_CHARGE = "philosophes"
FENETRE_CHARGE = "fenetre"
DEFAULT_CHARGES = (ALEA_LCG_CHARGE, PHILOSOPHES_CHARGE, FENETRE_CHARGE)

ALEA_LCG_SIZES = (4, 8, 12, 16, 20, 24)
PHILOSOPHES_COUNTS = (3, 4, 5, 6, 7)
FENETRE_CASES = ((4, 10), (8, 10), (16, 10))
FENETRE_ENCODAGES = ("absolu", "relatif")

# Fil rouge pins: |R_n| for the philosophers charge (WORKLOG fragments
# and docs/exemple-fil-rouge). Checked at generation time - the shared
# state file is only valid if the pin holds.
PHILOSOPHES_STATE_COUNTS = {3: 20, 4: 56, 5: 152, 6: 416, 7: 1136}

# alea-lcg pins (popcount of the deterministic LCG table, same
# constants as benches/micro.rs M1-8). The larger sizes stay
# cross-checked between tools only.
ALEA_LCG_MINTERM_PINS = {4: 7, 12: 2074}

PENSE = 0
AFFAME = 1
MANGE = 2
LIBRE = 0
TENUE = 1
PRECEDENTE = 2

LISAGE_RUNNER_COMMAND = (
    "cargo",
    "run",
    "--release",
    "--quiet",
    "--package",
    "lisage-bench",
    "--bin",
    "confrontation",
    "--",
)


@dataclasses.dataclass(frozen=True)
class Mesure:
    """One runner output line, parsed and typed (protocol pinned in lisage)."""

    outil: str
    variante: str
    charge: str
    parametres: dict[str, str]
    encodage: str | None
    temps_ms: float
    rss_max_ko: int
    noeuds: int
    verifie: int


@dataclasses.dataclass(frozen=True)
class Cas:
    """One (tool, charge, arguments) case to run once per repetition."""

    outil: str
    charge: str
    arguments: tuple[str, ...]
    titre: str


@dataclasses.dataclass(frozen=True)
class Agrege:
    """Median aggregation over the repetitions of one measured group."""

    outil: str
    charge: str
    cle_parametres: tuple[tuple[str, str], ...]
    encodage: str | None
    variante: str
    temps_median_ms: float
    temps_min_ms: float
    temps_max_ms: float
    rss_median_ko: int
    noeuds: int
    verifie: int
    repetitions: int


def etat_philosophes_valide(*, mot: Sequence[int]) -> bool:
    """True when no two adjacent philosophers eat (circularly)."""
    nombre = len(mot)
    return all(
        not (mot[index] == MANGE and mot[(index + 1) % nombre] == MANGE) for index in range(nombre)
    )


def fourchette_gauche(*, mot: Sequence[int], index: int) -> int:
    """Derived fork state: held when eating, else the eating neighbour's fork.

    The fork of philosopher ``index`` is TENUE when they eat
    (``mot[index] == MANGE``); PRECEDENTE when the previous philosopher
    eats (the fork shared with them is taken); LIBRE otherwise.
    """
    nombre = len(mot)
    if mot[index] == MANGE:
        return TENUE
    if mot[(index - 1) % nombre] == MANGE:
        return PRECEDENTE
    return LIBRE


def generer_etats_etendus(*, nombre_philosophes: int) -> list[list[int]]:
    """Enumerate R_n: interleaved ``pc_i left_i`` states over valid words.

    Deterministic enumeration (lexicographic over
    ``{PENSE, AFFAME, MANGE}**n``), so the shared file is byte-stable
    across runs and the pin check is meaningful.
    """
    etats: list[list[int]] = []
    for mot in itertools.product((PENSE, AFFAME, MANGE), repeat=nombre_philosophes):
        if not etat_philosophes_valide(mot=mot):
            continue
        etat: list[int] = []
        for index in range(nombre_philosophes):
            etat.append(mot[index])
            etat.append(fourchette_gauche(mot=mot, index=index))
        etats.append(etat)
    return etats


def serialiser_fichier_etats(*, nombre_philosophes: int, etats: Sequence[Sequence[int]]) -> str:
    """Render the shared extended-state file: ``n <n>`` then one state per line."""
    lignes = [f"n {nombre_philosophes}"]
    lignes.extend(" ".join(str(valeur) for valeur in etat) for etat in etats)
    return "\n".join(lignes) + "\n"


def parse_mesure(*, ligne: str) -> Mesure:
    """Parse one protocol JSON line into a typed Mesure; ValueError on drift."""
    brut = json.loads(ligne)
    cles_obligatoires = (
        "outil",
        "variante",
        "charge",
        "parametres",
        "temps_ms",
        "rss_max_ko",
        "noeuds",
        "verifie",
    )
    for cle in cles_obligatoires:
        if cle not in brut:
            raise ValueError(f"clé absente : {cle}")
    parametres = {str(cle): str(valeur) for cle, valeur in brut["parametres"].items()}
    return Mesure(
        outil=str(brut["outil"]),
        variante=str(brut["variante"]),
        charge=str(brut["charge"]),
        parametres=parametres,
        encodage=str(brut["encodage"]) if "encodage" in brut else None,
        temps_ms=float(brut["temps_ms"]),
        rss_max_ko=int(brut["rss_max_ko"]),
        noeuds=int(brut["noeuds"]),
        verifie=int(brut["verifie"]),
    )


def cle_parametres(*, mesure: Mesure) -> tuple[tuple[str, str], ...]:
    """Order-independent parameters key, for grouping across tools."""
    return tuple(sorted(mesure.parametres.items()))


def construire_cas(
    *,
    outil: str,
    charges: Sequence[str],
    chemin_etats: Callable[[int], str],
) -> list[Cas]:
    """Build one tool's case grid: every (charge, arguments) to measure.

    The window charge carries the encoding axis for lisage only
    (``absolu`` and ``relatif`` are two different lisage diagrams); the
    witnesses have a single encoding, so they measure one case.
    """
    cas: list[Cas] = []
    if ALEA_LCG_CHARGE in charges:
        cas.extend(
            Cas(
                outil=outil,
                charge=ALEA_LCG_CHARGE,
                arguments=(str(taille),),
                titre=f"alea-lcg {taille}",
            )
            for taille in ALEA_LCG_SIZES
        )
    if PHILOSOPHES_CHARGE in charges:
        cas.extend(
            Cas(
                outil=outil,
                charge=PHILOSOPHES_CHARGE,
                arguments=(str(nombre), chemin_etats(nombre)),
                titre=f"philosophes {nombre}",
            )
            for nombre in PHILOSOPHES_COUNTS
        )
    if FENETRE_CHARGE in charges:
        for taille, nombre_bases in FENETRE_CASES:
            if outil == LISAGE_TOOL_NAME:
                for encodage in FENETRE_ENCODAGES:
                    cas.append(
                        Cas(
                            outil=outil,
                            charge=FENETRE_CHARGE,
                            arguments=(str(taille), str(nombre_bases), encodage),
                            titre=f"fenetre {taille} {nombre_bases} {encodage}",
                        )
                    )
            else:
                cas.append(
                    Cas(
                        outil=outil,
                        charge=FENETRE_CHARGE,
                        arguments=(str(taille), str(nombre_bases)),
                        titre=f"fenetre {taille} {nombre_bases}",
                    )
                )
    return cas


def construire_commande(*, cas: Cas, repertoire_scratch: Path) -> Sequence[str]:
    """Build the fresh-process command for one case; witnesses run in Docker.

    lisage runs from the repository root; witnesses run through
    ``docker run`` with the run's scratch directory mounted at
    ``/confrontation`` (the state files are passed as container paths).
    """
    if cas.outil == LISAGE_TOOL_NAME:
        return (*LISAGE_RUNNER_COMMAND, cas.charge, *cas.arguments)
    if cas.outil in WITNESS_IMAGES:
        return (
            "docker",
            "run",
            "--rm",
            f"--volume={repertoire_scratch.resolve()}:{DOCKER_SCRATCH_MOUNT}",
            WITNESS_IMAGES[cas.outil],
            WITNESS_BINARIES[cas.outil],
            cas.charge,
            *cas.arguments,
        )
    raise ValueError(f"outil inconnu : {cas.outil}")


def executer_cas(*, cas: Cas, repertoire_scratch: Path) -> Mesure:
    """Run one case once in a fresh process and parse its single JSON line."""
    commande = construire_commande(cas=cas, repertoire_scratch=repertoire_scratch)
    try:
        resultat = subprocess.run(
            commande,
            capture_output=True,
            text=True,
            check=False,
            cwd=REPOSITORY_ROOT,
            timeout=RUN_TIMEOUT_SECONDS,
        )
    except FileNotFoundError as erreur:
        print(f"Error: command not found for {cas.outil}: {erreur}", file=sys.stderr)
        raise SystemExit(EXIT_CODE_PROTOCOL_FAILURE) from erreur
    except subprocess.TimeoutExpired as erreur:
        print(f"Error: {cas.titre} timed out on {cas.outil}.", file=sys.stderr)
        raise SystemExit(EXIT_CODE_PROTOCOL_FAILURE) from erreur
    if resultat.returncode != 0:
        print(
            f"Error: {cas.outil} failed on {cas.titre} "
            f"(exit {resultat.returncode}): {resultat.stderr.strip()}",
            file=sys.stderr,
        )
        raise SystemExit(EXIT_CODE_PROTOCOL_FAILURE)
    lignes = [ligne for ligne in resultat.stdout.splitlines() if ligne.strip()]
    if len(lignes) != 1:
        print(
            f"Error: {cas.outil} on {cas.titre} printed {len(lignes)} lines, "
            "expected exactly one JSON line.",
            file=sys.stderr,
        )
        raise SystemExit(EXIT_CODE_PROTOCOL_FAILURE)
    try:
        return parse_mesure(ligne=lignes[0])
    except ValueError as erreur:
        print(
            f"Error: unparsable line from {cas.outil} on {cas.titre}: {erreur}",
            file=sys.stderr,
        )
        raise SystemExit(EXIT_CODE_PROTOCOL_FAILURE) from erreur


def verifier_coherence(*, mesures: Sequence[Mesure]) -> None:
    """Cross-tool check: identical ``verifie`` per (charge, parameters).

    The ``encodage`` axis is deliberately excluded: both lisage
    encodings of the window charge, and any witness encoding, must
    agree on the count - only the node counts may differ.
    """
    par_charge: dict[tuple[str, tuple[tuple[str, str], ...]], set[int]] = {}
    for mesure in mesures:
        cle = (mesure.charge, cle_parametres(mesure=mesure))
        par_charge.setdefault(cle, set()).add(mesure.verifie)
    incoherences = {cle: valeurs for cle, valeurs in par_charge.items() if len(valeurs) != 1}
    if incoherences:
        for (charge, parametres), valeurs in sorted(incoherences.items()):
            print(
                f"Error: {charge} {dict(parametres)} disagrees on verifie: {sorted(valeurs)}.",
                file=sys.stderr,
            )
        raise SystemExit(EXIT_CODE_PROTOCOL_FAILURE)


def verifier_epingles(*, mesures: Sequence[Mesure]) -> None:
    """Check the pins the driver knows independently of the runners."""
    for mesure in mesures:
        if mesure.charge == PHILOSOPHES_CHARGE:
            attendu = PHILOSOPHES_STATE_COUNTS.get(int(mesure.parametres["n"]))
            if attendu is not None and mesure.verifie != attendu:
                print(
                    f"Error: philosophes {mesure.parametres['n']} pin is {attendu}, "
                    f"a runner reported {mesure.verifie}.",
                    file=sys.stderr,
                )
                raise SystemExit(EXIT_CODE_PROTOCOL_FAILURE)
        if mesure.charge == ALEA_LCG_CHARGE:
            attendu = ALEA_LCG_MINTERM_PINS.get(int(mesure.parametres["n"]))
            if attendu is not None and mesure.verifie != attendu:
                print(
                    f"Error: alea-lcg {mesure.parametres['n']} pin is {attendu}, "
                    f"a runner reported {mesure.verifie}.",
                    file=sys.stderr,
                )
                raise SystemExit(EXIT_CODE_PROTOCOL_FAILURE)
        if mesure.charge == FENETRE_CHARGE:
            attendu = int(mesure.parametres["taille"]) * int(mesure.parametres["nb_bases"])
            if mesure.verifie != attendu:
                print(
                    f"Error: fenetre {mesure.parametres} must verify {attendu}, "
                    f"a runner reported {mesure.verifie}.",
                    file=sys.stderr,
                )
                raise SystemExit(EXIT_CODE_PROTOCOL_FAILURE)


def agreger(*, mesures: Sequence[Mesure]) -> list[Agrege]:
    """Median over repetitions per (tool, charge, parameters, encoding)."""
    groupes: dict[tuple[str, str, tuple[tuple[str, str], ...], str | None], list[Mesure]] = {}
    for mesure in mesures:
        cle = (mesure.outil, mesure.charge, cle_parametres(mesure=mesure), mesure.encodage)
        groupes.setdefault(cle, []).append(mesure)
    agreges: list[Agrege] = []
    for (outil, charge, parametres, encodage), groupe in groupes.items():
        temps = sorted(mesure.temps_ms for mesure in groupe)
        agreges.append(
            Agrege(
                outil=outil,
                charge=charge,
                cle_parametres=parametres,
                encodage=encodage,
                variante=groupe[0].variante,
                temps_median_ms=statistics.median(temps),
                temps_min_ms=temps[0],
                temps_max_ms=temps[-1],
                rss_median_ko=int(statistics.median(mesure.rss_max_ko for mesure in groupe)),
                noeuds=groupe[0].noeuds,
                verifie=groupe[0].verifie,
                repetitions=len(groupe),
            )
        )
    return agreges


def rendu_parametres(*, agrege: Agrege) -> str:
    """Render the parameters column of the report."""
    return ", ".join(f"{cle}={valeur}" for cle, valeur in agrege.cle_parametres)


def ordre_rapport(*, agrege: Agrege) -> tuple[tuple[tuple[str, str], ...], int, str]:
    """Sort key: parameters, then tool order, then encoding."""
    position_outil = (
        TOOL_ORDER.index(agrege.outil) if agrege.outil in TOOL_ORDER else len(TOOL_ORDER)
    )
    return (agrege.cle_parametres, position_outil, agrege.encodage or "")


def rendre_rapport(*, agreges: Sequence[Agrege], repetitions: int) -> str:
    """Render the aggregated Markdown report, one table per charge."""
    lignes = ["# Confrontation lisage / témoins", ""]
    lignes.append(
        f"Répétitions par cas : {repetitions}, processus frais à chaque mesure "
        "(protocole : docs/bench-confrontation-temoins-et-mesures.md)."
    )
    lignes.append("")
    for charge in DEFAULT_CHARGES:
        charge_agreges = [agrege for agrege in agreges if agrege.charge == charge]
        if not charge_agreges:
            continue
        lignes.append(f"## {charge}")
        lignes.append("")
        lignes.append(
            "| outil | variante | paramètres | encodage | temps médian (ms) |"
            " temps min (ms) | temps max (ms) | RSS médiane (Ko) |"
            " nœuds | vérifié |"
        )
        lignes.append("|---|---|---|---|---|---|---|---|---|---|")
        for agrege in sorted(charge_agreges, key=lambda agrege: ordre_rapport(agrege=agrege)):
            lignes.append(
                f"| {agrege.outil} | {agrege.variante} |"
                f" {rendu_parametres(agrege=agrege)} |"
                f" {agrege.encodage or '-'} | {agrege.temps_median_ms:.3f} |"
                f" {agrege.temps_min_ms:.3f} | {agrege.temps_max_ms:.3f} |"
                f" {agrege.rss_median_ko} | {agrege.noeuds} | {agrege.verifie} |"
            )
        lignes.append("")
    return "\n".join(lignes)


def parse_liste_outils(*, brute: str) -> tuple[str, ...]:
    """Parse the comma-separated tool list, in the given order."""
    outils = tuple(outil.strip() for outil in brute.split(",") if outil.strip())
    inconnus = [outil for outil in outils if outil not in TOOL_ORDER]
    if inconnus:
        print(
            f"Error: unknown tool(s) {inconnus}; known tools: {list(TOOL_ORDER)}.",
            file=sys.stderr,
        )
        raise SystemExit(EXIT_CODE_PROTOCOL_FAILURE)
    return outils


def parse_liste_charges(*, brute: str) -> tuple[str, ...]:
    """Parse the comma-separated charge list."""
    charges = tuple(charge.strip() for charge in brute.split(",") if charge.strip())
    inconnues = [charge for charge in charges if charge not in DEFAULT_CHARGES]
    if inconnues:
        print(
            f"Error: unknown charge(s) {inconnues}; known charges: {list(DEFAULT_CHARGES)}.",
            file=sys.stderr,
        )
        raise SystemExit(EXIT_CODE_PROTOCOL_FAILURE)
    return charges


def preparer_fichiers_etats(*, repertoire_scratch: Path) -> dict[int, Path]:
    """Generate and pin-check the shared philosophers state files."""
    fichiers: dict[int, Path] = {}
    for nombre in PHILOSOPHES_COUNTS:
        etats = generer_etats_etendus(nombre_philosophes=nombre)
        attendu = PHILOSOPHES_STATE_COUNTS[nombre]
        if len(etats) != attendu:
            print(
                f"Error: |R_{nombre}| = {len(etats)}, pin is {attendu} - the "
                "generation drifted, refusing to bench on a broken input.",
                file=sys.stderr,
            )
            raise SystemExit(EXIT_CODE_PROTOCOL_FAILURE)
        chemin = repertoire_scratch / f"philosophes-{nombre}.states"
        chemin.write_text(
            serialiser_fichier_etats(nombre_philosophes=nombre, etats=etats),
            encoding="utf-8",
        )
        fichiers[nombre] = chemin
    return fichiers


def runner(*, arguments: argparse.Namespace) -> int:
    """Run the confrontation bench and write the run directory."""
    outils = parse_liste_outils(brute=arguments.outils)
    charges = parse_liste_charges(brute=arguments.charges)
    horodatage = datetime.datetime.now(datetime.UTC).strftime(RUN_TIMESTAMP_FORMAT)
    repertoire_course = REPOSITORY_ROOT / RUN_DIRECTORY_NAME / horodatage
    repertoire_scratch = repertoire_course / SCRATCH_DIRECTORY_NAME
    repertoire_scratch.mkdir(parents=True, exist_ok=True)

    fichiers_etats: dict[int, Path] = {}
    if PHILOSOPHES_CHARGE in charges:
        fichiers_etats = preparer_fichiers_etats(repertoire_scratch=repertoire_scratch)

    cas_total: list[Cas] = []
    for outil in outils:

        def chemin_etats(nombre: int, *, outil: str = outil) -> str:
            """State-file path as the runner sees it: host or container."""
            if outil == LISAGE_TOOL_NAME:
                return str(fichiers_etats[nombre])
            return f"{DOCKER_SCRATCH_MOUNT}/{fichiers_etats[nombre].name}"

        cas_total.extend(construire_cas(outil=outil, charges=charges, chemin_etats=chemin_etats))

    mesures: list[Mesure] = []
    total = len(cas_total) * arguments.repetitions
    effectue = 0
    for cas in cas_total:
        for _ in range(arguments.repetitions):
            mesure = executer_cas(cas=cas, repertoire_scratch=repertoire_scratch)
            mesures.append(mesure)
            effectue += 1
            print(f"[{effectue}/{total}] {cas.outil} {cas.titre} ok")

    verifier_epingles(mesures=mesures)
    verifier_coherence(mesures=mesures)
    agreges = agreger(mesures=mesures)

    chemin_lignes = repertoire_course / RAW_LINES_FILE_NAME
    with chemin_lignes.open("w", encoding="utf-8") as fichier:
        for mesure in mesures:
            fichier.write(json.dumps(dataclasses.asdict(mesure)) + "\n")
    chemin_rapport = repertoire_course / REPORT_FILE_NAME
    chemin_rapport.write_text(
        rendre_rapport(agreges=agreges, repetitions=arguments.repetitions),
        encoding="utf-8",
    )

    print(f"Coherence check passed across {len(outils)} tool(s).")
    print(f"Raw lines: {chemin_lignes}")
    print(f"Report: {chemin_rapport}")
    return EXIT_CODE_SUCCESS


def register(subparsers: argparse._SubParsersAction[argparse.ArgumentParser]) -> None:
    """Register the bench-confrontation subcommand (ADR-0021)."""
    subparser = subparsers.add_parser(
        SUBCOMMAND_NAME,
        help="bench lisage against the external DD witnesses (ADR-0102)",
        description=(
            "Run the time/memory confrontation bench: same charges, fresh "
            "process per repetition, one JSON line per measure, cross-tool "
            "verification. Witnesses run in local Docker containers."
        ),
    )
    subparser.add_argument(
        "--outils",
        default=",".join(DEFAULT_TOOLS),
        help=f"comma-separated tools among {list(TOOL_ORDER)} (default: %(default)s)",
    )
    subparser.add_argument(
        "--charges",
        default=",".join(DEFAULT_CHARGES),
        help=(f"comma-separated charges among {list(DEFAULT_CHARGES)} (default: %(default)s)"),
    )
    subparser.add_argument(
        "--repetitions",
        type=int,
        default=DEFAULT_REPETITIONS,
        help="fresh-process repetitions per case (default: %(default)s)",
    )
    subparser.set_defaults(runner=runner)
