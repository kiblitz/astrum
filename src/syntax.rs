use ratatui::style::Color;
use std::collections::HashMap;
use std::path::Path;
use streaming_iterator::StreamingIterator;
use tree_sitter::{InputEdit, Language, Parser, Point, Query, QueryCursor, Tree};

/// A single styled text segment for rendering.
#[derive(Debug, Clone)]
pub struct StyledSegment {
    pub fg: Color,
    pub text: String,
}

/// Per-buffer highlight data: one vec of styled segments per line.
pub type LineHighlights = Vec<Vec<StyledSegment>>;

/// Information about a buffer edit, used for incremental tree-sitter parsing.
#[derive(Debug, Clone)]
pub struct EditInfo {
    pub start_byte: usize,
    pub old_end_byte: usize,
    pub new_end_byte: usize,
    pub start_position: (usize, usize), // (row, col_bytes)
    pub old_end_position: (usize, usize),
    pub new_end_position: (usize, usize),
}

/// Per-buffer syntax state: parser, tree, and query.
struct BufferSyntaxState {
    parser: Parser,
    tree: Option<Tree>,
    query: Query,
}

struct LanguageEntry {
    language: Language,
    highlights_query: &'static str,
}

struct LanguageRegistry {
    by_extension: HashMap<&'static str, &'static str>,
    languages: HashMap<&'static str, LanguageEntry>,
}

impl LanguageRegistry {
    fn new() -> Self {
        let mut reg = Self {
            by_extension: HashMap::new(),
            languages: HashMap::new(),
        };

        reg.register(
            "rust",
            tree_sitter_rust::LANGUAGE.into(),
            tree_sitter_rust::HIGHLIGHTS_QUERY,
            &["rs"],
        );
        reg.register(
            "python",
            tree_sitter_python::LANGUAGE.into(),
            tree_sitter_python::HIGHLIGHTS_QUERY,
            &["py", "pyi"],
        );
        reg.register(
            "javascript",
            tree_sitter_javascript::LANGUAGE.into(),
            tree_sitter_javascript::HIGHLIGHT_QUERY,
            &["js", "mjs", "cjs", "jsx"],
        );
        reg.register(
            "json",
            tree_sitter_json::LANGUAGE.into(),
            tree_sitter_json::HIGHLIGHTS_QUERY,
            &["json"],
        );
        reg.register(
            "toml",
            tree_sitter_toml_ng::LANGUAGE.into(),
            tree_sitter_toml_ng::HIGHLIGHTS_QUERY,
            &["toml"],
        );
        reg.register(
            "c",
            tree_sitter_c::LANGUAGE.into(),
            tree_sitter_c::HIGHLIGHT_QUERY,
            &["c", "h"],
        );
        reg.register(
            "cpp",
            tree_sitter_cpp::LANGUAGE.into(),
            tree_sitter_cpp::HIGHLIGHT_QUERY,
            &["cpp", "cc", "cxx", "hpp", "hxx", "hh"],
        );

        reg
    }

    fn register(
        &mut self,
        name: &'static str,
        language: Language,
        highlights_query: &'static str,
        extensions: &[&'static str],
    ) {
        self.languages.insert(name, LanguageEntry {
            language,
            highlights_query,
        });
        for ext in extensions {
            self.by_extension.insert(ext, name);
        }
    }

    fn detect(&self, path: Option<&Path>) -> Option<&'static str> {
        let ext = path?.extension()?.to_str()?;
        self.by_extension.get(ext).copied()
    }

    fn get(&self, name: &str) -> Option<&LanguageEntry> {
        self.languages.get(name)
    }
}

/// Maps tree-sitter capture names to colors (One Dark palette).
fn capture_color(name: &str) -> Color {
    match name {
        // Keywords — purple
        "keyword" | "keyword.control" | "keyword.function" | "keyword.operator"
        | "keyword.import" | "keyword.return" | "keyword.exception"
        | "keyword.conditional" | "keyword.repeat" | "keyword.storage"
        | "keyword.directive" | "keyword.modifier" | "keyword.type"
        | "keyword.coroutine" => Color::Rgb(198, 120, 221),

        // Strings — green
        "string" | "string.special" | "string.escape" | "string.regex"
        | "string.special.path" | "string.special.url"
        | "string.special.symbol" => Color::Rgb(152, 195, 121),

        // Comments — muted gray
        "comment" | "comment.documentation" => Color::Rgb(92, 99, 112),

        // Functions — blue
        "function" | "function.builtin" | "function.call"
        | "function.macro" | "function.method" | "function.method.call"
        => Color::Rgb(97, 175, 239),

        // Types — yellow
        "type" | "type.builtin" | "type.definition" | "type.qualifier"
        => Color::Rgb(229, 192, 123),

        // Variables — soft white (default text)
        "variable" | "variable.member" | "variable.parameter"
        => Color::Rgb(171, 178, 191),
        // Built-in variables — red
        "variable.builtin" => Color::Rgb(224, 108, 117),

        // Constants & numbers — dark cyan
        "constant" | "constant.builtin" | "constant.macro"
        => Color::Rgb(86, 182, 194),
        "number" | "number.float" => Color::Rgb(209, 154, 102),
        "boolean" => Color::Rgb(209, 154, 102),

        // Operators — soft cyan
        "operator" => Color::Rgb(86, 182, 194),

        // Punctuation — dimmed so it recedes
        "punctuation" | "punctuation.bracket" | "punctuation.delimiter"
        | "punctuation.special" => Color::Rgb(130, 137, 151),

        // Attributes — yellow
        "attribute" | "attribute.builtin" => Color::Rgb(229, 192, 123),

        "label" => Color::Rgb(229, 192, 123),

        // Modules/namespaces — yellow
        "module" | "namespace" => Color::Rgb(229, 192, 123),

        // Constructors — blue
        "constructor" => Color::Rgb(97, 175, 239),

        // Properties — red
        "property" => Color::Rgb(224, 108, 117),

        // Tags (HTML/XML) — red
        "tag" => Color::Rgb(224, 108, 117),

        "embedded" => Color::Rgb(171, 178, 191),

        // Default text
        _ => Color::Rgb(171, 178, 191),
    }
}

/// Central highlight engine with incremental tree-sitter parsing.
pub struct HighlightEngine {
    registry: LanguageRegistry,
    states: HashMap<usize, BufferSyntaxState>,
}

impl HighlightEngine {
    pub fn new() -> Self {
        Self {
            registry: LanguageRegistry::new(),
            states: HashMap::new(),
        }
    }

    /// Full parse — called on file open, undo, redo, or when no tree exists.
    pub fn parse_full(
        &mut self,
        buffer_id: usize,
        source: &str,
        path: Option<&Path>,
    ) -> LineHighlights {
        let lang_name = match self.registry.detect(path) {
            Some(name) => name,
            None => return plain_highlights(source),
        };
        let entry = match self.registry.get(lang_name) {
            Some(e) => e,
            None => return plain_highlights(source),
        };

        let state = self.states.entry(buffer_id).or_insert_with(|| {
            let mut parser = Parser::new();
            parser.set_language(&entry.language).expect("language version mismatch");
            let query = Query::new(&entry.language, entry.highlights_query)
                .expect("invalid highlights query");
            BufferSyntaxState { parser, tree: None, query }
        });

        let tree = state.parser.parse(source, None).unwrap();
        let highlights = compute_highlights(source, &tree, &state.query);
        state.tree = Some(tree);
        highlights
    }

    /// Incremental parse — called after each edit with the EditInfo.
    pub fn parse_incremental(
        &mut self,
        buffer_id: usize,
        source: &str,
        edit: &EditInfo,
    ) -> Option<LineHighlights> {
        let state = self.states.get_mut(&buffer_id)?;
        let old_tree = state.tree.as_mut()?;

        old_tree.edit(&InputEdit {
            start_byte: edit.start_byte,
            old_end_byte: edit.old_end_byte,
            new_end_byte: edit.new_end_byte,
            start_position: Point {
                row: edit.start_position.0,
                column: edit.start_position.1,
            },
            old_end_position: Point {
                row: edit.old_end_position.0,
                column: edit.old_end_position.1,
            },
            new_end_position: Point {
                row: edit.new_end_position.0,
                column: edit.new_end_position.1,
            },
        });

        let new_tree = state.parser.parse(source, Some(old_tree))?;
        let highlights = compute_highlights(source, &new_tree, &state.query);
        state.tree = Some(new_tree);
        Some(highlights)
    }

    /// Remove syntax state for a closed buffer.
    pub fn remove_buffer(&mut self, buffer_id: usize) {
        self.states.remove(&buffer_id);
    }
}

fn compute_highlights(source: &str, tree: &Tree, query: &Query) -> LineHighlights {
    let mut cursor = QueryCursor::new();
    let mut matches = cursor.matches(query, tree.root_node(), source.as_bytes());

    // Collect all highlight spans: (start_byte, end_byte, capture_index).
    let capture_names = query.capture_names();
    let mut spans: Vec<(usize, usize, &str)> = Vec::new();
    while let Some(m) = {
        matches.advance();
        matches.get()
    } {
        for capture in m.captures {
            let name = &capture_names[capture.index as usize];
            let node = capture.node;
            spans.push((node.start_byte(), node.end_byte(), name));
        }
    }
    // Sort by start position; for overlapping spans, prefer shorter (more specific).
    spans.sort_by_key(|s| (s.0, s.1 - s.0));

    build_line_highlights(source, &spans)
}

fn build_line_highlights(source: &str, spans: &[(usize, usize, &str)]) -> LineHighlights {
    let default_fg = Color::Rgb(171, 178, 191);
    let mut result = Vec::new();
    let mut span_idx = 0;

    let mut line_start_byte = 0;
    for line in source.split('\n') {
        let line_end_byte = line_start_byte + line.len();
        let mut segments: Vec<StyledSegment> = Vec::new();
        let mut pos = line_start_byte;

        // Advance span_idx past spans that end before this line.
        while span_idx < spans.len() && spans[span_idx].1 <= line_start_byte {
            span_idx += 1;
        }

        // Walk through spans overlapping this line.
        let mut si = span_idx;
        while si < spans.len() && spans[si].0 < line_end_byte {
            let (s_start, s_end, name) = spans[si];
            let visible_start = s_start.max(line_start_byte);
            let visible_end = s_end.min(line_end_byte);

            if visible_start > pos {
                // Gap before this span — default color.
                let text = &source[pos..visible_start];
                if !text.is_empty() {
                    segments.push(StyledSegment {
                        fg: default_fg,
                        text: text.to_string(),
                    });
                }
            }

            if visible_start >= pos && visible_end > visible_start {
                let text = &source[visible_start..visible_end];
                if !text.is_empty() {
                    segments.push(StyledSegment {
                        fg: capture_color(name),
                        text: text.to_string(),
                    });
                }
                pos = visible_end;
            }

            si += 1;
        }

        // Remainder of line after last span.
        if pos < line_end_byte {
            let text = &source[pos..line_end_byte];
            if !text.is_empty() {
                segments.push(StyledSegment {
                    fg: default_fg,
                    text: text.to_string(),
                });
            }
        }

        result.push(segments);
        // +1 for the '\n' delimiter.
        line_start_byte = line_end_byte + 1;
    }

    result
}

/// Produce plain (unstyled) highlights for files with no grammar.
fn plain_highlights(source: &str) -> LineHighlights {
    let fg = Color::Rgb(171, 178, 191);
    source
        .lines()
        .map(|line| {
            vec![StyledSegment {
                fg,
                text: line.to_string(),
            }]
        })
        .collect()
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
