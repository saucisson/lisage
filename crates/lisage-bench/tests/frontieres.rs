//! Test de frontières (M0-2) : vérifie que les dépendances entre crates
//! respectent le graphe unidirectionnel de la proposition § 8.
//!
//! Graphe autorisé :
//!   lisage-lang      → (rien)
//!   lisage-algebra   → lisage-lang
//!   lisage-plan      → lisage-lang, lisage-algebra
//!   lisage-core      → lisage-lang, lisage-algebra
//!   lisage-backend   → lisage-core, lisage-lang
//!   lisage-bench     → lisage-core, lisage-lang, lisage-algebra, lisage-plan
//!   lisage-liseuse   → lisage-core, lisage-lang

use std::collections::BTreeMap;
use std::path::Path;

const CRATES_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/..");

/// Graphe des dépendances autorisées (crate → dépendances internes permises).
const ALLOWED: &[(&str, &[&str])] = &[
    ("lisage-lang", &[]),
    ("lisage-algebra", &["lisage-lang"]),
    ("lisage-plan", &["lisage-lang", "lisage-algebra"]),
    ("lisage-core", &["lisage-lang", "lisage-algebra"]),
    ("lisage-backend", &["lisage-core", "lisage-lang"]),
    (
        "lisage-bench",
        &[
            "lisage-core",
            "lisage-lang",
            "lisage-algebra",
            "lisage-plan",
        ],
    ),
    ("lisage-liseuse", &["lisage-core", "lisage-lang"]),
];

fn parse_dependencies(crate_name: &str) -> Vec<String> {
    let manifest = Path::new(CRATES_DIR).join(crate_name).join("Cargo.toml");
    let content = std::fs::read_to_string(&manifest)
        .unwrap_or_else(|error| panic!("lecture de {}: {error}", manifest.display()));
    let mut dependencies = Vec::new();
    let mut in_deps = false;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed == "[dependencies]" {
            in_deps = true;
            continue;
        }
        if trimmed.starts_with('[') {
            in_deps = false;
            continue;
        }
        if in_deps && trimmed.starts_with("lisage-") {
            let name = trimmed.split('=').next().unwrap().trim().to_string();
            dependencies.push(name);
        }
    }
    dependencies
}

#[test]
fn test_frontieres_dependances_unidirectionnelles() {
    let mut failures = Vec::new();
    for (crate_name, allowed) in ALLOWED {
        let actual = parse_dependencies(crate_name);
        for dep in &actual {
            if !allowed.contains(&dep.as_str()) {
                failures.push(format!(
                    "{crate_name} dépend de {dep}, non autorisé (attendu: {allowed:?})"
                ));
            }
        }
        for expected in *allowed {
            if !actual.iter().any(|d| d == expected) {
                failures.push(format!(
                    "{crate_name} devrait dépendre de {expected} (dépendances: {actual:?})"
                ));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "Violations de frontières:\n{}",
        failures.join("\n")
    );
}

/// Traits d'interface dont la déclaration doit porter les bornes `Send`/`Sync`
/// figées dès M1-0 (feuille de route § 10) : (fichier relatif à crates/, motif).
const SIGNATURE_BOUNDS: &[(&str, &str)] = &[
    (
        "lisage-algebra/src/interface.rs",
        "pub trait KeyAlgebra: Send + Sync",
    ),
    (
        "lisage-algebra/src/interface.rs",
        "pub trait Controller: Send + Sync",
    ),
    (
        "lisage-algebra/src/interface.rs",
        "pub trait Pool: Send + Sync",
    ),
    (
        "lisage-core/src/observer.rs",
        "pub trait Observer: Send + Sync",
    ),
];

#[test]
fn test_frontieres_signatures_send_sync() {
    let mut failures = Vec::new();
    for (relative_path, expected_declaration) in SIGNATURE_BOUNDS {
        let path = Path::new(CRATES_DIR).join(relative_path);
        let content = std::fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("lecture de {}: {error}", path.display()));
        if !content.contains(expected_declaration) {
            failures.push(format!(
                "{relative_path} doit déclarer `{expected_declaration}`"
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "Bornes Send/Sync manquantes sur les traits d'interface:\n{}",
        failures.join("\n")
    );
}

#[test]
fn test_sept_crates_presentes() {
    let expected: BTreeMap<String, ()> = ALLOWED
        .iter()
        .map(|(name, _)| (name.to_string(), ()))
        .collect();
    let crates_path = Path::new(CRATES_DIR);
    let mut found = BTreeMap::new();
    for entry in std::fs::read_dir(crates_path).unwrap() {
        let entry = entry.unwrap();
        let file_name = entry.file_name();
        if entry.path().is_dir()
            && let Some(name) = file_name.to_str()
            && name.starts_with("lisage-")
        {
            found.insert(name.to_string(), ());
        }
    }
    assert_eq!(found, expected, "Les 7 crates lisage-* doivent exister");
}
