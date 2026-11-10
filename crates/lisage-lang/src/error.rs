//! Politique d'erreurs du niveau 1 (feuille de route § 10) : contrairement
//! aux invariants internes de `core/`/`algebra/` (asserts), le niveau `lang/`
//! remonte des `Result` typés, causes préservées, rien n'est avalé.

/// Nom de clé du langage, type distinct à la frontière (ADR-0006).
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct KeyName(pub String);

/// Erreur typée du langage de référence.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum LangError {
    /// `lookup` sur une clé hors de l'univers déclaré.
    KeyOutsideUniverse(KeyName),
    /// Valeur d'un type inattendu à un point du calcul.
    TypeMismatch {
        expected: &'static str,
        found: &'static str,
    },
    /// Univers vide : aucun niveau de présence à balayer.
    EmptyUniverse,
}

impl std::fmt::Display for LangError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LangError::KeyOutsideUniverse(key) => {
                write!(formatter, "clé hors de l'univers déclaré: {}", key.0)
            }
            LangError::TypeMismatch { expected, found } => {
                write!(formatter, "type attendu {expected}, trouvé {found}")
            }
            LangError::EmptyUniverse => write!(formatter, "univers vide"),
        }
    }
}

impl std::error::Error for LangError {}
