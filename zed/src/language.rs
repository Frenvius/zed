use crate::settings::{Theme, ThemeMap};
use parking_lot::Mutex;
use rust_embed::RustEmbed;
use serde::Deserialize;
use std::{path::Path, str, sync::Arc};
use tree_sitter::{Language as Grammar, Query};
pub use tree_sitter::{Parser, Tree};

#[derive(RustEmbed)]
#[folder = "languages"]
pub struct LanguageDir;

#[derive(Default, Deserialize)]
pub struct LanguageConfig {
    pub name: String,
    pub path_suffixes: Vec<String>,
}

pub struct Language {
    pub config: LanguageConfig,
    pub grammar: Grammar,
    pub highlight_query: Query,
    pub theme_mapping: Mutex<ThemeMap>,
}

pub struct LanguageRegistry {
    languages: Vec<Arc<Language>>,
}

impl Language {
    pub fn theme_mapping(&self) -> ThemeMap {
        self.theme_mapping.lock().clone()
    }

    pub fn set_theme(&self, theme: &Theme) {
        *self.theme_mapping.lock() = ThemeMap::new(self.highlight_query.capture_names(), theme);
    }
}

impl LanguageRegistry {
    pub fn new() -> Self {
        let grammar = tree_sitter_rust::language();
        let rust_config = toml::from_slice(&LanguageDir::get("rust/config.toml").unwrap()).unwrap();
        let rust_language = Language {
            config: rust_config,
            grammar,
            highlight_query: Self::load_query(grammar, "rust/highlights.scm"),
            theme_mapping: Mutex::new(ThemeMap::default()),
        };

        Self {
            languages: vec![Arc::new(rust_language)],
        }
    }

    pub fn set_theme(&self, theme: &Theme) {
        for language in &self.languages {
            language.set_theme(theme);
        }
    }

    pub fn select_language(&self, path: impl AsRef<Path>) -> Option<&Arc<Language>> {
        let path = path.as_ref();
        let filename = path.file_name().and_then(|name| name.to_str());
        let extension = path.extension().and_then(|name| name.to_str());
        let path_suffixes = [extension, filename];
        self.languages.iter().find(|language| {
            language
                .config
                .path_suffixes
                .iter()
                .any(|suffix| path_suffixes.contains(&Some(suffix.as_str())))
        })
    }

    fn load_query(grammar: tree_sitter::Language, path: &str) -> Query {
        Query::new(
            grammar,
            str::from_utf8(LanguageDir::get(path).unwrap().as_ref()).unwrap(),
        )
        .unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_select_language() {
        let grammar = tree_sitter_rust::language();
        let registry = LanguageRegistry {
            languages: vec![
                Arc::new(Language {
                    config: LanguageConfig {
                        name: "Rust".to_string(),
                        path_suffixes: vec!["rs".to_string()],
                        ..Default::default()
                    },
                    grammar,
                    highlight_query: Query::new(grammar, "").unwrap(),
                    theme_mapping: Default::default(),
                }),
                Arc::new(Language {
                    config: LanguageConfig {
                        name: "Make".to_string(),
                        path_suffixes: vec!["Makefile".to_string(), "mk".to_string()],
                        ..Default::default()
                    },
                    grammar,
                    highlight_query: Query::new(grammar, "").unwrap(),
                    theme_mapping: Default::default(),
                }),
            ],
        };

        // matching file extension
        assert_eq!(
            registry.select_language("zed/lib.rs").map(get_name),
            Some("Rust")
        );
        assert_eq!(
            registry.select_language("zed/lib.mk").map(get_name),
            Some("Make")
        );

        // matching filename
        assert_eq!(
            registry.select_language("zed/Makefile").map(get_name),
            Some("Make")
        );

        // matching suffix that is not the full file extension or filename
        assert_eq!(registry.select_language("zed/cars").map(get_name), None);
        assert_eq!(registry.select_language("zed/a.cars").map(get_name), None);
        assert_eq!(registry.select_language("zed/sumk").map(get_name), None);

        fn get_name(language: &Arc<Language>) -> &str {
            language.config.name.as_str()
        }
    }
}
