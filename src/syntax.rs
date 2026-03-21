use ratatui::style::Color;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use syntect::highlighting::{HighlightIterator, HighlightState, Highlighter, ThemeSet};
use syntect::parsing::{ParseState, ScopeStack, SyntaxSet};
use tokio::sync::mpsc;

/// A single styled text segment for rendering.
#[derive(Debug, Clone)]
pub struct StyledSegment {
    pub fg: Color,
    pub text: String,
}

/// Per-buffer highlight data: one vec of styled segments per line.
pub type LineHighlights = Vec<Vec<StyledSegment>>;

/// Message sent from background highlight tasks back to the editor.
pub struct HighlightResult {
    pub buffer_id: usize,
    pub highlights: LineHighlights,
}

/// Owns the syntax/theme sets (behind Arc for cheap cloning to background tasks)
/// and dispatches highlighting work onto the tokio blocking pool.
pub struct HighlightEngine {
    syntax_set: Arc<SyntaxSet>,
    theme_set: Arc<ThemeSet>,
    theme_name: String,
    tx: mpsc::Sender<HighlightResult>,
}

impl HighlightEngine {
    pub fn new(
        theme_name: &str,
        tx: mpsc::Sender<HighlightResult>,
    ) -> Self {
        Self {
            syntax_set: Arc::new(SyntaxSet::load_defaults_newlines()),
            theme_set: Arc::new(ThemeSet::load_defaults()),
            theme_name: theme_name.to_string(),
            tx,
        }
    }

    /// Spawn a background task to highlight the given text.
    /// Results are sent via the channel; the editor polls the receiver.
    pub fn request_highlight(
        &self,
        buffer_id: usize,
        text: String,
        path: Option<PathBuf>,
    ) {
        let ss = Arc::clone(&self.syntax_set);
        let ts = Arc::clone(&self.theme_set);
        let theme_name = self.theme_name.clone();
        let tx = self.tx.clone();

        tokio::task::spawn_blocking(move || {
            let highlights = compute_highlights(&ss, &ts, &theme_name, &text, path.as_deref());
            // If the receiver is dropped, just discard.
            let _ = tx.blocking_send(HighlightResult {
                buffer_id,
                highlights,
            });
        });
    }
}

fn compute_highlights(
    syntax_set: &SyntaxSet,
    theme_set: &ThemeSet,
    theme_name: &str,
    text: &str,
    path: Option<&Path>,
) -> LineHighlights {
    let syntax = detect_syntax(syntax_set, path);
    let theme = match theme_set.themes.get(theme_name) {
        Some(t) => t,
        None => theme_set.themes.values().next().unwrap(),
    };
    let highlighter = Highlighter::new(theme);
    let mut parse_state = ParseState::new(syntax);
    let mut highlight_state = HighlightState::new(&highlighter, ScopeStack::new());

    let mut result = Vec::new();

    for line in text.lines() {
        // syntect expects lines with newlines
        let line_with_nl = format!("{}\n", line);
        let ops = parse_state
            .parse_line(&line_with_nl, syntax_set)
            .unwrap_or_default();
        let iter = HighlightIterator::new(&mut highlight_state, &ops, &line_with_nl, &highlighter);

        let segments: Vec<StyledSegment> = iter
            .map(|(style, text)| {
                let display = text.replace('\n', "").replace('\r', "");
                StyledSegment {
                    fg: Color::Rgb(style.foreground.r, style.foreground.g, style.foreground.b),
                    text: display,
                }
            })
            .collect();

        result.push(segments);
    }

    result
}

fn detect_syntax<'a>(syntax_set: &'a SyntaxSet, path: Option<&Path>) -> &'a syntect::parsing::SyntaxReference {
    if let Some(path) = path {
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            if let Some(s) = syntax_set.find_syntax_by_extension(ext) {
                return s;
            }
        }
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            if let Some(s) = syntax_set.find_syntax_by_extension(name) {
                return s;
            }
        }
    }
    syntax_set.find_syntax_plain_text()
}

/// Cache that stores highlight results per buffer.
#[derive(Debug, Default)]
pub struct HighlightCache {
    cache: HashMap<usize, LineHighlights>,
}

impl HighlightCache {
    pub fn new() -> Self {
        Self {
            cache: HashMap::new(),
        }
    }

    pub fn insert(&mut self, buffer_id: usize, highlights: LineHighlights) {
        self.cache.insert(buffer_id, highlights);
    }

    pub fn get(&self, buffer_id: usize) -> Option<&LineHighlights> {
        self.cache.get(&buffer_id)
    }

    pub fn invalidate(&mut self, buffer_id: usize) {
        self.cache.remove(&buffer_id);
    }
}
