"""English-only verification over the repository (issues #104, #110).

The repository is English-only by charter (M6, issue #104): the accented
character grep of the first ``make english-only`` only caught part of the
problem, because written French can be accent-free (``defaut``,
``serialiser``, ``noeud``). This checker keeps the accented signal and
adds an unaccented-French signal, calibrated on the M6-renamed tree:

- accented characters: every file in scope;
- French identifier stems: code files only (identifiers, comments,
  docstrings), never the contents of Rust or Python string literals:
  they carry wire bytes (the JSONL v3 keys and channel tags, issue
  #158) and the benchmark CLI names, which are data, not prose;
- French stop-words: prose only. In Rust they are looked for in
  comments alone, so no English identifier ever trips them.

In Markdown, fenced code blocks and inline code spans are skipped, so
the bilingual UI labels of ADR-0107 and wire-format examples can be
quoted verbatim inside code formatting.

Allowed, on purpose and reviewed per PR:

- ``docs/glossary.md`` holds the FR->EN mapping terms themselves;
- ``docs/audit-french-inventory.md`` quotes the inventoried French;
- ``WORKLOG.md`` keeps its pre-M6 entries in French (its header forbids
  rewriting them);
- ``.github/pr-body-*.md`` are historical artifacts quoting human
  directives verbatim;
- ``bench-results/**`` archives published measurement outputs;
- lock files are machine-generated.

The subcommand takes two shapes (the second one ported from
project-template ADR-0008 N4, PR 21):

- ``python -m scripts english-only`` and ``make english-only``: scan
  the whole scope, print one line per offender, exit 1 as soon as one
  is found - the CI and ``make check`` front;
- ``python -m scripts english-only --subject-file PATH``: scan only
  the first line of that file, accents and stop-words, exit 1 on an
  offender - the commit-msg hook shape: a commit subject is an
  artifact like any other, and the hook wiring lives in
  ``.pre-commit-config.yaml``.

Entry point (ADR-0021): ``python -m scripts english-only`` or
``make english-only``. Exit codes: 0 clean, 1 offenders found (printed
one per line, with the reason and the matching word), 2 protocol
failure (an unreadable commit message file).

The package targets Python 3.15: the sources use unparenthesized
multi-exception ``except`` clauses, which parse only from Python 3.14,
so nothing under 3.14 can import ``scripts``.
"""

import argparse
import re
import sys
import tokenize
from io import StringIO
from pathlib import Path

REPOSITORY_ROOT = Path(__file__).resolve().parent.parent

# The accented signal, built from code points so this file itself stays
# accent-free: only the letters that written French uses. Broader
# ranges (Latin-1 Supplement, Latin Extended-A) flag legitimate
# non-French quotations, such as Danish bibliographic names.
_ACCENT_LOWER = "".join(
    chr(cp)
    for cp in (
        0xE0,
        0xE2,
        0xE4,
        0xE7,
        0xE9,
        0xE8,
        0xEA,
        0xEB,
        0xEE,
        0xEF,
        0xF4,
        0xF9,
        0xFB,
        0xFC,
        0xFF,
        0x153,
        0xE6,
    )
)
_ACCENT_UPPER = _ACCENT_LOWER.upper()
ACCENTS = _ACCENT_LOWER + _ACCENT_UPPER
ACCENT_RE = re.compile(f"[{re.escape(ACCENTS)}]")

# French function words, as whole words only: they catch accent-free
# French prose in comments, docstrings and plain-text config. Calibrated
# on the M6 tree: none of these words appears in the English code or the
# English configuration of the repository.
_STOP_WORD_LEXICON = (
    "le la les un une des du de en et ou est sont dans sur pour avec par "
    "pas mais donc car quand si nous vous ici"
)
STOP_WORDS = _STOP_WORD_LEXICON.split()

# French identifier stems left over by the M6 renames (issues #103 to
# #110), matched against snake-case and CamelCase parts of identifiers
# in code files only. ``value``, ``image`` and ``charge`` are
# deliberately absent: they are legitimate English words.
_FRENCH_LEXICON = """
    absente absents absolu absolue absolues accepte
    actif actifs adopte affectation aire aleatoire aleatoires aligne
    alignement alignes aller alloues amorce anchorcomposee ancienne ancres
    anneau antisymetrique appartenance applique appliquee appliquer apres arbre
    arcmaloriente arene arenes atelier atome atomes atteignable atteignables
    atteint attendu attendue attendus attraper attribut aucun aucune
    autodetectee autorisee autre autres aux aval avant avec
    balayage balayageunivers balise banc bande barre boolou bord
    borne bornes boucle bout branche brut canal canonicite
    canonique canoniques caractere cardinaux carte cartes cellules chaine
    champ champs charger chemin cherche cherches choix cible
    classe classeur classique cle clef clefs cles cloture
    cloturee coherents collectes comme comparaison compatibilite complementaire composee
    composees comptage compte compter compteur constante constantes construire
    construit construits contenu contexte contextephiout contextualiser contrainte contraintes
    contrat contre controleur copie corps couleur couleurs courante
    cout couture coutures couvre croissance croissante curseur dans
    de debut decalage decalages decalee decremente defaut defauts
    deja demi densite dependances depliee deplier depuis dernier
    derniere des descripteur deserialise deserialiser dessine dessiner dessous
    determinisme deterministe deux diagonale diagramme differentes differentiel directe
    disperser distinctes distincts domainatomes domaine doublee doublon droite
    du duree echappe echappee echapper echelle echoue ecrit
    ecriture ecriturememory ecritures effet egal egale egales egalite
    emet empreinte empreintes encodage encre ensemble ensembles entier
    entiers entree entrees enveloppe epingle epinglee epingles equilibree
    equivaut erreur espace essai est et etape etat
    etats etendu etendus etiqueter etiquette etiquettes etrangere evalue
    evenement evenementielle evenements exacte exactes executer existe expectede
    explicite exportee extraire extrait extraits facteur factice famille
    faux fenetre fenetres fermante fermer fermeture feuille feuilles
    fichier fichiers fil filtre filtrer fin fixe fleche
    fois fonction fond forme frise frontiere frontieres garde
    gauche glessee glissante glisse glissee glisses grande greffe
    greffer greffon gross hachage hauteur herite herites hierarchie
    horloges hors idempotente identifiant identique identite idsaleatoires inchangee
    incluse incoherent incoherente inconnu indexee indice inferieure integrale
    intercalage intercale interne internement internements invalide invalidee invalides
    jeton jetons joue jusqua justifie keypresentes largeur lecture
    lectures legales legende les libelle libre ligne lignes
    lineaire liste litterale locaux longueur majore mal malformees
    mange mangent mangeurs manquant manquante marge marquage marquages
    marque marquebalayage marquecanonique marqueencodage marqueenumeration
    marqueorder marqueur matrice
    meme memoire memoires memorynonautorisee mesure mesurer metriques miroir
    miroirs modele modifie modifies montant mots multiplicateur multiplicite
    naif naturel nb neuf neutre niveau niveaux noeud
    noeuds nom nombre noms nul nulle numero obtenu
    obtenue obtenus octet octets ombre ordersinconvertibles ordre oriente
    orientee ou ouvert ouverte ouvrante ouvrir paire paires
    panneau parametres parametresinvalides parametresoracle parametressaturation
    partage partages parties
    pas pasunepermutation pave personnalise petit piste pli pliage
    plier plis plusieurs poids pondere ponderee ponderees porteur
    porteurs pour pourcentage precedent precedente predicat predire premier
    premiere prendre presentes prevu prevue produit profilee profondeur
    propagee propriete proprietes quand racine rang rationnel rebords
    recree recu recus redondance reduire reduit reetiquetage reetiquete
    reetiquetes refusee registre regle regles rejet rejete rejette
    relatif relatifs rendre reordonner repli replicanonique reproductibles requetes
    requise reseau resoudre reste restreint restreinte resultat resultats
    retour retrilles retrillie rouge sans satisfait saturer seconde
    semantique semianneau sensibilisation sensibilise sensibilisee sept serialisation serialise
    serialisee serialiser seuil seul seule si socle socles
    sortie sorties sous split() stateetendu statique stockage strategie
    strategieaval successeur suivant suivante suivants superieure supporte sur
    surcharge synthetise synthetiseur taille temoin temoins terminaison terminaux
    termine texte textes tir tirer titre touche tous
    tout toutes transducteur transfert transparente tri triee triees
    trier triptyque trois tronque tronques trouvees typage typee
    typees un une unidirectionnelles unite univers valeur valeurs
    valide validee valider variableendouble variante vaut verifie verifiee
    verifieechoue vers versionnee violee visite visites vivantes vivants
    voie vrai vu vue vus
"""
FRENCH_IDENTIFIERS = frozenset(_FRENCH_LEXICON.split())

# The scan scope of the charter: root Markdown, docs in full, Rust
# sources and manifests, tool sources, project drivers, deployment text,
# historical PR bodies and .gitignore.
SCOPE_GLOBS = (
    "*.md",
    ".gitignore",
    "docs/**",
    "crates/**/*.rs",
    "crates/**/*.toml",
    "tools/*/src/**/*.py",
    "scripts/**/*.py",
    "docker/**",
    ".github/**/*.md",
)

# Deliberately allowed to hold French (see the module docstring).
ALLOWED_EXACT = frozenset(
    {
        "WORKLOG.md",
        "docs/glossary.md",
        "docs/audit-french-inventory.md",
    }
)
ALLOWED_BEGINNINGS = (
    (".github", "pr-body-"),
    ("bench-results",),
)

_TEXT_SUFFIXES = frozenset({".png", ".jpg", ".jpeg", ".gif", ".ico", ".woff", ".woff2", ".ttf"})

_FENCE_RE = re.compile(r"(?ms)^(```|~~~)[^\n]*?$\n.*?^\1[^\n]*?$")
InlineSpan = re.compile(r"`[^`\n]*`")
# A Rust string prefix: b/r combinations, raw hashes, opening quote.
_RUST_STRING_PREFIX = re.compile(r"(?:b?r|r|b)(?P<hashes>#{0,9})\"")
Identifier_RE = re.compile(r"[A-Za-z_][A-Za-z_0-9]*")
_PART_SPLIT_RE = re.compile(r"[_0-9]+|(?<=[a-z])(?=[A-Z])")

COMMIT_SUBJECT_ANCHOR = Path("<commit-subject>")


def is_allowed(relative: Path) -> bool:
    """Tell whether a scanned file is deliberately allowed to hold French."""
    if relative.as_posix() in ALLOWED_EXACT:
        return True
    if relative.suffix == ".lock":
        return True
    parts = relative.parts
    if parts[0] == ".github" and parts[1].startswith("pr-body-"):
        return True
    return any(parts[: len(prefix)] == prefix for prefix in ALLOWED_BEGINNINGS)


def markdown_without_code_spans(text: str) -> str:
    """Strip fenced code blocks and inline code spans from Markdown.

    Bilingual UI labels and wire-format examples are quoted inside code
    formatting on purpose (ADR-0107): the checker must not flag them.
    """
    stripped = _FENCE_RE.sub(" ", text)
    return InlineSpan.sub(" ", stripped)


def _string_end(text: str, opening_quote: int, hashes: int) -> int:
    """Return the index just past a Rust string literal.

    Raw strings close on ``"`` followed by as many ``#`` as they open
    with; plain strings honour the backslash escape and end at the first
    unescaped ``"``.
    """
    if hashes:
        closing = '"' + "#" * hashes
        position = text.find(closing, opening_quote + 1)
        return len(text) if position < 0 else position + len(closing)
    position = opening_quote + 1
    while position < len(text):
        char = text[position]
        if char == "\\":
            position += 2
            continue
        if char == '"':
            return position + 1
        position += 1
    return len(text)


def rust_without_strings(text: str) -> str:
    """Drop Rust string literals, keep code and comments.

    String literals carry wire bytes (JSONL v3 keys, channel tags,
    issue #158) and benchmark CLI names: they are data, not prose, and
    stay out of the French scan. Byte strings, raw strings (including
    ``r#"..."#``) and char literals are handled; lifetimes are preserved.
    """
    out: list[str] = []
    index = 0
    length = len(text)
    while index < length:
        char = text[index]
        prefix = _RUST_STRING_PREFIX.match(text, index)
        if prefix is not None:
            opening = prefix.end() - 1
            out.append(" ")
            index = _string_end(text, opening, len(prefix.group("hashes")))
            continue
        if char == '"':
            out.append(" ")
            index = _string_end(text, index, 0)
            continue
        if char == "'" and re.match(r"'(\\.|[^'\\\n])'", text[index:]):
            closing = re.match(r"'(\\.|[^'\\\n])'", text[index:])
            out.append(" ")
            index += closing.end() if closing is not None else 2
            continue
        if char == "/" and text[index : index + 2] == "//":
            end = text.find("\n", index)
            end = length if end < 0 else end
            out.append(text[index:end])
            index = end
            continue
        if char == "/" and text[index : index + 2] == "/*":
            end = text.find("*/", index)
            end = length if end < 0 else end + 2
            out.append(text[index:end])
            index = end
            continue
        out.append(char)
        index += 1
    return "".join(out)


def python_corpus(source: str) -> tuple[list[str], list[str], list[str]]:
    """Split a Python source into (names, comments, strings).

    Names (identifiers) are scanned for French stems, comments for
    stop-words and stems, strings for accents only: a literal may
    legitimately quote a French label or a wire-format key.
    """
    names: list[str] = []
    comments: list[str] = []
    strings: list[str] = []
    for token in tokenize.generate_tokens(StringIO(source).readline):
        if token.type == tokenize.NAME:
            names.append(token.string)
        elif token.type == tokenize.COMMENT:
            comments.append(token.string)
        elif token.type == tokenize.STRING:
            strings.append(token.string)
    return names, comments, strings


def _line_of(text: str, position: int) -> int:
    """Return the 1-based line number of a character position."""
    return text.count("\n", 0, position) + 1


def _first_accent(text: str) -> tuple[int, str] | None:
    """Return the position and evidence of the first accented character."""
    match = ACCENT_RE.search(text)
    if match is None:
        return None
    start = match.start()
    snippet = text[max(0, start - 20) : start + 21].replace("\n", " ")
    return start, f"accented character: ...{snippet}..."


def _first_stop_word(text: str) -> tuple[int, str] | None:
    """Return the position of the first French function word, if any.

    Boundaries exclude letters, digits and hyphens: a function word
    never abuts a digit in French prose, and this keeps pinned commit
    SHAs (``...6de127b...``) in Dockerfiles from tripping the scan.
    """
    for word in STOP_WORDS:
        match = re.search(rf"(?<![A-Za-z0-9-]){word}(?![A-Za-z0-9-])", text)
        if match is not None:
            return match.start(), f"French stop-word: {word}"
    return None


def _find_french_part(identifier: str) -> str | None:
    """Return the first French stem found in a snake/CamelCase part."""
    parts = _PART_SPLIT_RE.split(identifier)
    for part in parts:
        if part.lower() in FRENCH_IDENTIFIERS:
            return f"French identifier: {identifier}"
    return None


def scan_corpus(
    relative: Path,
    text: str,
    *,
    stop_words: bool,
    identifiers: bool,
) -> Offender | None:
    """Scan one text corpus; return the first offender found, if any."""
    finding = _first_accent(text)
    if finding is not None:
        position, evidence = finding
        return Offender(relative, _line_of(text, position), evidence)
    if stop_words:
        finding = _first_stop_word(text)
        if finding is not None:
            position, evidence = finding
            return Offender(relative, _line_of(text, position), evidence)
    if identifiers:
        for match in Identifier_RE.finditer(text):
            finding_text = _find_french_part(match.group(0))
            if finding_text is not None:
                return Offender(relative, _line_of(text, match.start()), finding_text)
    return None


class Offender:
    """One French finding: file, line and what matched."""

    def __init__(self, relative: Path, line: int, evidence: str) -> None:
        """Store the finding: file, 1-based line and matching evidence."""
        self.relative = relative
        self.line = line
        self.evidence = evidence

    def __str__(self) -> str:
        """Render the finding as ``path:line: evidence``."""
        location = self.relative.as_posix()
        if self.line:
            location = f"{location}:{self.line}"
        return f"{location}: {self.evidence}"


def _blank(text: str) -> str:
    """Blank a non-comment span, preserving newlines (line numbers)."""
    return "".join(char if char == "\n" else " " for char in text)


def rust_comments_in_place(text: str) -> str:
    """Keep only comments, blanking code and string literals in place.

    French function words are looked for in prose only: Rust comments.
    Scanning the whole code corpus makes them match inside legitimate
    identifiers. Blanking in place preserves offender line numbers.
    """
    out: list[str] = []
    index = 0
    length = len(text)
    while index < length:
        char = text[index]
        prefix = _RUST_STRING_PREFIX.match(text, index)
        if prefix is not None:
            opening = prefix.end() - 1
            end = _string_end(text, opening, len(prefix.group("hashes")))
            out.append(_blank(text[index:end]))
            index = end
            continue
        if char == '"':
            end = _string_end(text, index, 0)
            out.append(_blank(text[index:end]))
            index = end
            continue
        if char == "'" and re.match(r"'(\\.|[^'\\\n])'", text[index:]):
            closing = re.match(r"'(\\.|[^'\\\n])'", text[index:])
            out.append(_blank(text[index : index + (closing.end() if closing is not None else 2)]))
            index += closing.end() if closing is not None else 2
            continue
        if char == "/" and text[index : index + 2] == "//":
            end = text.find("\n", index)
            end = length if end < 0 else end
            out.append(text[index:end])
            index = end
            continue
        if char == "/" and text[index : index + 2] == "/*":
            end = text.find("*/", index)
            end = length if end < 0 else end + 2
            out.append(text[index:end])
            index = end
            continue
        out.append(_blank(char))
        index += 1
    return "".join(out)


def scan_rust(relative: Path, text: str) -> Offender | None:
    """Scan a Rust file: identifiers and comments, strings aside.

    Accents and French identifier stems are looked for over the code
    corpus without string literals, and French function words over the
    comments only, so that English identifiers (``from_le_bytes``)
    do not trip it, and wire-format examples quoted in backticks are
    data, never French prose.
    """
    corpus = rust_without_strings(text)
    # Wire-format examples quoted in backticks are data, not identifiers.
    corpus = InlineSpan.sub(lambda m: " " * len(m.group(0)), corpus)
    offender = scan_corpus(relative, corpus, stop_words=False, identifiers=True)
    if offender is not None:
        return offender
    return scan_corpus(relative, rust_comments_in_place(text), stop_words=True, identifiers=False)


def scan_python(relative: Path, text: str) -> Offender | None:
    """Scan a Python file through its three corpora."""
    try:
        names, comments, strings = python_corpus(text)
    except SyntaxError, tokenize.TokenError, IndentationError:
        return scan_corpus(relative, text, stop_words=True, identifiers=False)
    offender = scan_corpus(relative, "\n".join(comments), stop_words=True, identifiers=True)
    if offender is not None:
        return offender
    offender = scan_corpus(relative, "\n".join(strings), stop_words=False, identifiers=False)
    if offender is not None:
        return offender
    return scan_corpus(relative, " ".join(names), stop_words=False, identifiers=True)


def scan_file(relative: Path) -> Offender | None:
    """Scan one in-scope file according to its type."""
    if relative.suffix in _TEXT_SUFFIXES:
        return None
    try:
        text = (REPOSITORY_ROOT / relative).read_text(encoding="utf-8")
    except OSError, UnicodeDecodeError:
        return None  # Binary or unreadable (logos, fixtures): out of scope.
    suffix = relative.suffix
    if suffix == ".rs":
        return scan_rust(relative, text)
    if suffix == ".py":
        return scan_python(relative, text)
    if suffix == ".md":
        return scan_corpus(
            relative, markdown_without_code_spans(text), stop_words=False, identifiers=False
        )
    # Plain-text config and deployment files: prose only, so stop-words apply
    # and identifiers do not.
    stop_words = (
        relative.name == ".gitignore"
        or relative.name.startswith("Dockerfile")
        or suffix in {".sh", ".c", ".h"}
    )
    return scan_corpus(relative, text, stop_words=stop_words, identifiers=False)


def iter_scope() -> list[Path]:
    """List every in-scope file, relative to the repository root."""
    seen: dict[Path, None] = {}
    for pattern in SCOPE_GLOBS:
        for path in REPOSITORY_ROOT.glob(pattern):
            if path.is_file():
                seen[path.relative_to(REPOSITORY_ROOT)] = None
    return list(seen)


def scan_repository() -> list[Offender]:
    """Scan the whole scope; return every offender found."""
    offenders: list[Offender] = []
    for relative in iter_scope():
        if is_allowed(relative):
            continue
        offender = scan_file(relative)
        if offender is not None:
            offenders.append(offender)
    return offenders


def scan_subject(*, subject: str) -> Offender | None:
    """Scan a commit subject: accented characters and French stop-words.

    The subject is prose, so the stem corpus stays out of it and
    identifiers with their own naming rules never apply.
    """
    finding = _first_accent(subject)
    if finding is None:
        finding = _first_stop_word(subject)
    if finding is None:
        return None
    position, evidence = finding
    return Offender(COMMIT_SUBJECT_ANCHOR, _line_of(subject, position), evidence)


def register(subparsers: argparse._SubParsersAction) -> None:
    """Register the english-only subcommand."""
    subparser = subparsers.add_parser(
        "english-only",
        help="Fail on French text over the repository (issues #104, #110).",
        description="Fail on French text over the repository (issues #104, #110).",
    )
    subparser.add_argument(
        "--subject-file",
        default=None,
        help="Commit-msg hook shape: scan only the first line of this file.",
    )
    subparser.set_defaults(runner=run)


def run(*, arguments: argparse.Namespace) -> int:
    """Print one line per offender; exit 1 as soon as one was found.

    The commit-msg hook shape (`--subject-file`) scans only the first
    line of that file instead of the whole repository.
    """
    subject_file = getattr(arguments, "subject_file", None)
    if subject_file is not None:
        try:
            message_text = Path(subject_file).read_text(encoding="utf-8")
        except OSError:
            print(
                f"Error: could not read the commit message file: {subject_file}.",
                file=sys.stderr,
            )
            return 2
        subject_lines = message_text.splitlines()
        subject = subject_lines[0] if subject_lines else ""
        subject_offender = scan_subject(subject=subject)
        if subject_offender is not None:
            print(f"english-only: {subject_offender}")
            print("english-only: the commit subject must be English (issues #104, #110).")
            return 1
        return 0
    offenders = scan_repository()
    for offender in offenders:
        print(f"english-only: {offender}")
    if offenders:
        print(
            f"english-only: {len(offenders)} offender(s): this repository is "
            "English-only by charter (issues #104, #110)."
        )
        return 1
    return 0


if __name__ == "__main__":  # Normally reached through ``python -m scripts``.
    sys.exit(run(arguments=argparse.Namespace()))
