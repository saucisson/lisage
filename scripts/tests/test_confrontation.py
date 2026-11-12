"""Pin the confrontation bench driver: generation, protocol, aggregation."""

from pathlib import Path

import pytest

from scripts.confrontation import (
    ALEA_LCG_CHARGE,
    CUDD_TOOL_NAME,
    FENETRE_CHARGE,
    LISAGE_TOOL_NAME,
    PHILOSOPHES_CHARGE,
    Cas,
    Mesure,
    agreger,
    cle_parametres,
    construire_cas,
    construire_commande,
    generer_etats_etendus,
    parse_mesure,
    rendre_rapport,
    serialiser_fichier_etats,
    verifier_coherence,
    verifier_epingles,
)


def mesure_de(
    *,
    outil: str = LISAGE_TOOL_NAME,
    variante: str = "bdd",
    charge: str = ALEA_LCG_CHARGE,
    parametres: dict[str, str] | None = None,
    encodage: str | None = None,
    temps_ms: float = 1.0,
    verifie: int = 7,
) -> Mesure:
    """Build a Mesure with defaults, for group-check tests."""
    return Mesure(
        outil=outil,
        variante=variante,
        charge=charge,
        parametres=parametres if parametres is not None else {"n": "4"},
        encodage=encodage,
        temps_ms=temps_ms,
        rss_max_ko=1024,
        noeuds=5,
        verifie=verifie,
    )


def test_generation_epingle_les_pins_du_fil_rouge() -> None:
    """|R_n| matches the WORKLOG pins for n=3..7."""
    assert len(generer_etats_etendus(nombre_philosophes=3)) == 20
    assert len(generer_etats_etendus(nombre_philosophes=4)) == 56
    assert len(generer_etats_etendus(nombre_philosophes=5)) == 152
    assert len(generer_etats_etendus(nombre_philosophes=6)) == 416
    assert len(generer_etats_etendus(nombre_philosophes=7)) == 1136


def test_generation_derive_les_fourchettes() -> None:
    """Eating philosopher holds their fork; the eating neighbour's is taken."""
    etats = generer_etats_etendus(nombre_philosophes=3)
    # Word (MANGE, PENSE, PENSE) is valid: state is intercalated
    # pc_0 left_0 pc_1 left_1 pc_2 left_2 = 2,1,0,2,0,0.
    assert [2, 1, 0, 2, 0, 0] in etats
    # No state with two circularly adjacent eaters.
    for etat in etats:
        mot = etat[0::2]
        assert all(
            not (mot[index] == 2 and mot[(index + 1) % 3] == 2) for index in range(3)
        )


def test_serialisation_du_fichier_partage() -> None:
    """The shared file starts with ``n <n>`` then one interleaved state per line."""
    contenu = serialiser_fichier_etats(
        nombre_philosophes=3,
        etats=[[0, 0, 0, 0, 0, 0], [2, 1, 0, 2, 0, 0]],
    )
    assert contenu.splitlines() == [
        "n 3",
        "0 0 0 0 0 0",
        "2 1 0 2 0 0",
    ]


def test_parse_mesure_accepte_une_ligne_protocole() -> None:
    """One protocol JSON line parses into a typed Mesure, encodage optional."""
    mesure = parse_mesure(
        ligne=(
            '{"outil":"lisage","variante":"mdd","charge":"fenetre",'
            '"parametres":{"taille":"4","nb_bases":"10"},'
            '"encodage":"absolu","temps_ms":0.123,"rss_max_ko":2048,'
            '"noeuds":12,"verifie":40}'
        )
    )
    assert mesure.outil == "lisage"
    assert mesure.encodage == "absolu"
    assert mesure.parametres == {"taille": "4", "nb_bases": "10"}
    sans_encodage = parse_mesure(
        ligne=(
            '{"outil":"cudd","variante":"bdd","charge":"fenetre",'
            '"parametres":{"taille":"4","nb_bases":"10"},'
            '"temps_ms":0.5,"rss_max_ko":4096,"noeuds":9,"verifie":40}'
        )
    )
    assert sans_encodage.encodage is None


def test_parse_mesure_rejette_une_ligne_a_la_derive() -> None:
    """A line missing a pinned key is a protocol drift, not a silent default."""
    with pytest.raises(ValueError, match="verifie"):
        parse_mesure(
            ligne=(
                '{"outil":"cudd","variante":"bdd","charge":"alea-lcg",'
                '"parametres":{"n":"4"},"temps_ms":0.5,"rss_max_ko":4096,"noeuds":9}'
            )
        )


def test_cle_parametres_est_independante_de_l_ordre() -> None:
    """Grouping keys must not depend on the runner's parameter order."""
    premiere = mesure_de(parametres={"taille": "4", "nb_bases": "10"})
    seconde = mesure_de(parametres={"nb_bases": "10", "taille": "4"})
    assert cle_parametres(mesure=premiere) == cle_parametres(mesure=seconde)


def test_grille_de_cas_porte_l_axe_encodage_de_lisage() -> None:
    """lisage measures both window encodings; the witness measures one."""
    cas_lisage = construire_cas(
        outil=LISAGE_TOOL_NAME,
        charges=(ALEA_LCG_CHARGE, PHILOSOPHES_CHARGE, FENETRE_CHARGE),
        chemin_etats=lambda nombre: f"/etats/philosophes-{nombre}.states",
    )
    cas_cudd = construire_cas(
        outil=CUDD_TOOL_NAME,
        charges=(ALEA_LCG_CHARGE, PHILOSOPHES_CHARGE, FENETRE_CHARGE),
        chemin_etats=lambda nombre: f"/confrontation/philosophes-{nombre}.states",
    )
    fenetres_lisage = [cas for cas in cas_lisage if cas.charge == FENETRE_CHARGE]
    fenetres_cudd = [cas for cas in cas_cudd if cas.charge == FENETRE_CHARGE]
    assert len(fenetres_lisage) == 6
    assert {cas.arguments[-1] for cas in fenetres_lisage} == {"absolu", "relatif"}
    assert len(fenetres_cudd) == 3
    assert len([cas for cas in cas_lisage if cas.charge == ALEA_LCG_CHARGE]) == 6
    assert len([cas for cas in cas_cudd if cas.charge == PHILOSOPHES_CHARGE]) == 5


def test_commande_lisage_est_un_processus_cargo_frais() -> None:
    """The lisage case runs the confrontation binary through cargo."""
    commande = construire_commande(
        cas=Cas(
            outil=LISAGE_TOOL_NAME,
            charge=PHILOSOPHES_CHARGE,
            arguments=("3", "/etats/philosophes-3.states"),
            titre="philosophes 3",
        ),
        repertoire_scratch=Path("/scratch"),
    )
    assert commande[0] == "cargo"
    assert commande[-3:] == ("philosophes", "3", "/etats/philosophes-3.states")


def test_commande_du_temoin_est_un_conteneur_docker() -> None:
    """The witness case mounts the run's scratch directory in its container."""
    commande = construire_commande(
        cas=Cas(
            outil=CUDD_TOOL_NAME,
            charge=PHILOSOPHES_CHARGE,
            arguments=("3", "/confrontation/philosophes-3.states"),
            titre="philosophes 3",
        ),
        repertoire_scratch=Path("/scratch"),
    )
    assert commande[:3] == ("docker", "run", "--rm")
    assert "--volume=/scratch:/confrontation" in commande
    assert "lisage-cudd:latest" in commande
    assert "cudd_bench" in commande


def test_coherence_passerait_sur_des_outils_d_accord() -> None:
    """Agreeing tools pass; one diverging verifie value fails the bench."""
    mesures = [
        mesure_de(outil=LISAGE_TOOL_NAME, verifie=7),
        mesure_de(outil=CUDD_TOOL_NAME, verifie=7),
        mesure_de(outil=LISAGE_TOOL_NAME, variante="mdd", charge=FENETRE_CHARGE,
                  parametres={"taille": "4", "nb_bases": "10"}, encodage="absolu",
                  verifie=40),
        mesure_de(outil=CUDD_TOOL_NAME, charge=FENETRE_CHARGE,
                  parametres={"taille": "4", "nb_bases": "10"}, verifie=40),
    ]
    verifier_coherence(mesures=mesures)  # no exception
    with pytest.raises(SystemExit):
        verifier_coherence(mesures=[*mesures, mesure_de(verifie=8)])


def test_epingles_refusent_les_valeurs_a_la_derive() -> None:
    """The known pins are checked against every runner line."""
    verifier_epingles(mesures=[mesure_de(verifie=7)])  # alea-lcg n=4 pin
    verifier_epingles(
        mesures=[
            mesure_de(variante="mdd", charge=PHILOSOPHES_CHARGE,
                      parametres={"n": "3"}, verifie=20)
        ]
    )
    verifier_epingles(
        mesures=[
            mesure_de(variante="mdd", charge=FENETRE_CHARGE,
                      parametres={"taille": "4", "nb_bases": "10"}, verifie=40)
        ]
    )
    with pytest.raises(SystemExit):
        verifier_epingles(mesures=[mesure_de(verifie=8)])
    with pytest.raises(SystemExit):
        verifier_epingles(
            mesures=[mesure_de(charge=PHILOSOPHES_CHARGE, parametres={"n": "3"},
                              verifie=21)]
        )
    with pytest.raises(SystemExit):
        verifier_epingles(
            mesures=[mesure_de(charge=FENETRE_CHARGE,
                              parametres={"taille": "4", "nb_bases": "10"},
                              verifie=39)]
        )


def test_agregation_est_mediane_par_groupe() -> None:
    """Medians group by (tool, charge, parameters, encoding) - encoding splits."""
    mesures = [
        mesure_de(temps_ms=3.0),
        mesure_de(temps_ms=1.0),
        mesure_de(temps_ms=2.0),
        mesure_de(outil=CUDD_TOOL_NAME, temps_ms=10.0),
        mesure_de(outil=CUDD_TOOL_NAME, temps_ms=4.0),
        mesure_de(variante="mdd", charge=FENETRE_CHARGE,
                  parametres={"taille": "4", "nb_bases": "10"}, encodage="absolu",
                  temps_ms=5.0),
        mesure_de(variante="mdd", charge=FENETRE_CHARGE,
                  parametres={"taille": "4", "nb_bases": "10"}, encodage="relatif",
                  temps_ms=6.0),
    ]
    agreges = agreger(mesures=mesures)
    par_encodage = {
        (agrege.outil, agrege.encodage): agrege for agrege in agreges
    }
    assert par_encodage[(LISAGE_TOOL_NAME, None)].temps_median_ms == 2.0
    assert par_encodage[(CUDD_TOOL_NAME, None)].temps_median_ms == 7.0
    assert par_encodage[(LISAGE_TOOL_NAME, "absolu")].temps_median_ms == 5.0
    assert par_encodage[(LISAGE_TOOL_NAME, "relatif")].temps_median_ms == 6.0


def test_rapport_rend_une_table_par_charge() -> None:
    """The report carries one section per charge and the encoding axis."""
    agreges = agreger(
        mesures=[
            mesure_de(temps_ms=1.0),
            mesure_de(outil=CUDD_TOOL_NAME, temps_ms=2.0),
            mesure_de(variante="mdd", charge=FENETRE_CHARGE,
                      parametres={"taille": "4", "nb_bases": "10"}, encodage="absolu",
                      verifie=40),
            mesure_de(variante="mdd", charge=FENETRE_CHARGE,
                      parametres={"taille": "4", "nb_bases": "10"}, encodage="relatif",
                      verifie=40),
        ]
    )
    rapport = rendre_rapport(agreges=agreges, repetitions=5)
    assert "## alea-lcg" in rapport
    assert "## fenetre" in rapport
    assert "absolu" in rapport
    assert rapport.count("| lisage | mdd |") == 2
