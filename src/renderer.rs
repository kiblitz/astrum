use crate::action::SearchDirection;
use crate::buffer::{Buffer, Cursor};
use crate::editor::{FindFileState, PaletteState, SearchState};
use crate::file_browser::{format_size, BrowserInputMode, FileBrowser};
use crate::input::Mode;
use crate::pane::{LayoutNode, PaneLayout, SplitDirection};
use crate::syntax::HighlightCache;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use std::collections::HashMap;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Tabs};
use ratatui::Frame;

pub struct Renderer;

impl Renderer {
    pub fn new() -> Self {
        Self
    }

    pub fn render(
        &self,
        frame: &mut Frame,
        buffers: &[Buffer],
        pane_layout: &PaneLayout,
        mode: &Mode,
        command_buffer: &str,
        status_message: &str,
        pending_keys: &str,
        pending_hints: &[(String, String)],
        highlight_cache: &HighlightCache,
        file_browsers: &HashMap<usize, FileBrowser>,
        palette: &Option<PaletteState>,
        find_file: &Option<FindFileState>,
        search_state: &SearchState,
        search_buffer: &str,
        search_direction: SearchDirection,
        visual_anchor: Option<(usize, usize)>,
        substitute_highlight: Option<(usize, usize, usize)>,
    ) {
        let size = frame.area();
        let has_overlay = find_file.is_some() || palette.is_some();

        // Compute active_buf_idx from pane_layout
        let active_buf_idx = pane_layout
            .active_pane()
            .buffer_id
            .and_then(|bid| buffers.iter().position(|b| b.id == bid))
            .unwrap_or(0);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // Tab bar
                Constraint::Min(1),   // Main area
                Constraint::Length(1), // Status line
                Constraint::Length(1), // Command line
            ])
            .split(size);

        // Tab bar
        if !buffers.is_empty() {
            self.render_tab_bar(frame, chunks[0], buffers, active_buf_idx);
        } else {
            let bar = Paragraph::new(Line::from(Span::styled(
                " astrum ",
                Style::default()
                    .fg(Color::Cyan)
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            )))
            .style(Style::default().bg(Color::DarkGray));
            frame.render_widget(bar, chunks[0]);
        }

        // Main area: always render panes, overlay file browsers on their panes
        let active_fb = file_browsers.get(&pane_layout.active_id);
        if !buffers.is_empty() || !pane_layout.is_single() {
            let show_cursor = !has_overlay;
            self.render_panes(frame, chunks[1], &pane_layout.root, pane_layout, buffers, highlight_cache, file_browsers, show_cursor, search_state, mode, visual_anchor, substitute_highlight);
            // Status line
            if let Some(fb) = active_fb {
                self.render_file_browser_status(frame, chunks[2], fb);
            } else {
                let active_pane = pane_layout.active_pane();
                if let Some(buf) = active_pane.buffer_id.and_then(|bid| buffers.iter().find(|b| b.id == bid)) {
                    self.render_status_line(frame, chunks[2], buf, &active_pane.cursor, mode);
                } else {
                    self.render_welcome_status(frame, chunks[2]);
                }
            }
        } else if let Some(fb) = active_fb {
            // Single pane, no buffers — file browser takes whole area
            self.render_file_browser(frame, chunks[1], fb);
            self.render_file_browser_status(frame, chunks[2], fb);
        } else {
            self.render_welcome(frame, chunks[1]);
            self.render_welcome_status(frame, chunks[2]);
        }

        self.render_command_line(
            frame,
            chunks[3],
            mode,
            command_buffer,
            status_message,
            pending_keys,
            pending_hints,
            active_fb,
            search_buffer,
            search_direction,
        );

        // Overlays
        if let Some(ff) = find_file {
            self.render_find_file(frame, size, ff);
        }
        if let Some(palette) = palette {
            self.render_palette(frame, size, palette);
        }

        // Cursor positioning: only set cursor when it should be visible.
        // Overlays hide the cursor, so skip positioning entirely for them.
        if !has_overlay {
            if *mode == Mode::Command {
                let x = chunks[3].x + 1 + command_buffer.len() as u16;
                frame.set_cursor_position((x, chunks[3].y));
            } else if *mode == Mode::Search {
                let x = chunks[3].x + 1 + search_buffer.len() as u16;
                frame.set_cursor_position((x, chunks[3].y));
            } else if let Some(fb) = active_fb {
                match fb.input_mode {
                    BrowserInputMode::Filter => {
                        let x = chunks[3].x + 1 + fb.filter.len() as u16;
                        frame.set_cursor_position((x, chunks[3].y));
                    }
                    BrowserInputMode::NewFile => {
                        let x = chunks[3].x + 10 + fb.new_file_name.len() as u16;
                        frame.set_cursor_position((x, chunks[3].y));
                    }
                    _ => {}
                }
            }
            // else: render_editor already set cursor position for active buffer,
            // or no buffer means no cursor (ratatui hides it by default).
        }
    }

    // -- Pane rendering (recursive) --

    fn render_panes(
        &self,
        frame: &mut Frame,
        area: Rect,
        node: &LayoutNode,
        pane_layout: &PaneLayout,
        buffers: &[Buffer],
        highlight_cache: &HighlightCache,
        file_browsers: &HashMap<usize, FileBrowser>,
        show_cursor: bool,
        search_state: &SearchState,
        mode: &Mode,
        visual_anchor: Option<(usize, usize)>,
        substitute_highlight: Option<(usize, usize, usize)>,
    ) {
        match node {
            LayoutNode::Leaf(pane_id) => {
                if let Some(pane) = pane_layout.pane_by_id(*pane_id) {
                    let is_active = pane.id == pane_layout.active_id;
                    let pane_fb = file_browsers.get(&pane.id);
                    let has_splits = !pane_layout.is_single();

                    // If splits exist, reserve 1 row for a per-pane status indicator
                    let (content_area, indicator_area) = if has_splits && area.height > 1 {
                        let content = Rect {
                            x: area.x,
                            y: area.y,
                            width: area.width,
                            height: area.height - 1,
                        };
                        let indicator = Rect {
                            x: area.x,
                            y: area.y + area.height - 1,
                            width: area.width,
                            height: 1,
                        };
                        (content, Some(indicator))
                    } else {
                        (area, None)
                    };

                    // Store the content height for viewport calculations.
                    pane.height.set(content_area.height);

                    // File browser renders on its owning pane
                    if let Some(fb) = pane_fb {
                        self.render_file_browser(frame, content_area, fb);
                    } else if let Some(buf) = pane.buffer_id.and_then(|bid| buffers.iter().find(|b| b.id == bid)) {
                        let pane_visual = if is_active && *mode == Mode::Visual { visual_anchor } else { None };
                        let pane_sub_hl = if is_active { substitute_highlight } else { None };
                        self.render_editor(frame, content_area, buf, &pane.cursor, pane.scroll_offset, is_active, highlight_cache, show_cursor, Some(search_state), pane_visual, pane_sub_hl);
                    } else {
                        self.render_welcome(frame, content_area);
                    }

                    // Per-pane indicator when splits exist
                    if let Some(ind_area) = indicator_area {
                        let buf_name = pane.buffer_id
                            .and_then(|bid| buffers.iter().find(|b| b.id == bid))
                            .map(|b| b.name.as_str())
                            .unwrap_or("[No File]");
                        let style = if is_active {
                            Style::default().fg(Color::Black).bg(Color::Blue)
                        } else {
                            Style::default().fg(Color::White).bg(Color::DarkGray)
                        };
                        let label = format!(" {} ", buf_name);
                        let pad = (ind_area.width as usize).saturating_sub(label.len());
                        let line = Line::from(vec![
                            Span::styled(label, style),
                            Span::styled(" ".repeat(pad), style),
                        ]);
                        frame.render_widget(Paragraph::new(line), ind_area);
                    }
                }
            }
            LayoutNode::Split { direction, children } => {
                let n = children.len() as u16;
                if n == 0 {
                    return;
                }
                let separators = n - 1;
                match direction {
                    SplitDirection::Vertical => {
                        let total_sep = separators; // 1 char each
                        if area.width < n + total_sep {
                            return;
                        }
                        let usable = area.width - total_sep;
                        let base_width = usable / n;
                        let extra = (usable % n) as usize;
                        let mut x_offset = area.x;

                        for (i, child) in children.iter().enumerate() {
                            // Distribute extra pixels to the first `extra` panes
                            let w = base_width + if i < extra { 1 } else { 0 };
                            let child_area = Rect {
                                x: x_offset,
                                y: area.y,
                                width: w,
                                height: area.height,
                            };
                            self.render_panes(frame, child_area, child, pane_layout, buffers, highlight_cache, file_browsers, show_cursor, search_state, mode, visual_anchor, substitute_highlight);
                            x_offset += w;

                            // Draw separator after each child except the last
                            if i + 1 < children.len() {
                                let sep_area = Rect {
                                    x: x_offset,
                                    y: area.y,
                                    width: 1,
                                    height: area.height,
                                };
                                self.render_split_border(frame, sep_area, SplitDirection::Vertical);
                                x_offset += 1;
                            }
                        }
                    }
                    SplitDirection::Horizontal => {
                        let total_sep = separators; // 1 char each
                        if area.height < n + total_sep {
                            return;
                        }
                        let usable = area.height - total_sep;
                        let base_height = usable / n;
                        let extra = (usable % n) as usize;
                        let mut y_offset = area.y;

                        for (i, child) in children.iter().enumerate() {
                            let h = base_height + if i < extra { 1 } else { 0 };
                            let child_area = Rect {
                                x: area.x,
                                y: y_offset,
                                width: area.width,
                                height: h,
                            };
                            self.render_panes(frame, child_area, child, pane_layout, buffers, highlight_cache, file_browsers, show_cursor, search_state, mode, visual_anchor, substitute_highlight);
                            y_offset += h;

                            if i + 1 < children.len() {
                                let sep_area = Rect {
                                    x: area.x,
                                    y: y_offset,
                                    width: area.width,
                                    height: 1,
                                };
                                self.render_split_border(frame, sep_area, SplitDirection::Horizontal);
                                y_offset += 1;
                            }
                        }
                    }
                }
            }
        }
    }

    fn render_split_border(&self, frame: &mut Frame, area: Rect, direction: SplitDirection) {
        let style = Style::default().fg(Color::DarkGray);
        match direction {
            SplitDirection::Vertical => {
                let mut lines = Vec::new();
                for _ in 0..area.height {
                    lines.push(Line::from(Span::styled("│", style)));
                }
                frame.render_widget(Paragraph::new(lines), area);
            }
            SplitDirection::Horizontal => {
                let line = Line::from(Span::styled(
                    "─".repeat(area.width as usize),
                    style,
                ));
                frame.render_widget(Paragraph::new(vec![line]), area);
            }
        }
    }

    // -- Welcome screen --

    fn render_welcome(&self, frame: &mut Frame, area: Rect) {
        let lines = vec![
            Line::from(""),
            Line::from(""),
            Line::from(""),
            Line::from(Span::styled(
                "    _        _                        ",
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(
                "   / \\   ___| |_ _ __ _   _ _ __ ___  ",
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(
                "  / _ \\ / __| __| '__| | | | '_ ` _ \\ ",
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(
                " / ___ \\\\__ \\ |_| |  | |_| | | | | | |",
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(
                "/_/   \\_\\___/\\__|_|   \\__,_|_| |_| |_|",
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "v0.1.0",
                Style::default().fg(Color::DarkGray),
            )),
            Line::from(""),
            Line::from(""),
            Line::from(vec![
                Span::styled("  SPC f f  ", Style::default().fg(Color::Yellow)),
                Span::styled("Browse files", Style::default().fg(Color::White)),
            ]),
            Line::from(vec![
                Span::styled("  :q       ", Style::default().fg(Color::Yellow)),
                Span::styled("Quit", Style::default().fg(Color::White)),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                "  Press SPC for leader key commands",
                Style::default().fg(Color::DarkGray),
            )),
        ];

        let paragraph = Paragraph::new(lines).alignment(Alignment::Center);
        frame.render_widget(paragraph, area);
    }

    fn render_welcome_status(&self, frame: &mut Frame, area: Rect) {
        let line = Line::from(vec![
            Span::styled(
                " NORMAL ",
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Blue)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                " Welcome ",
                Style::default().fg(Color::White).bg(Color::DarkGray),
            ),
            Span::styled(
                " ".repeat((area.width as usize).saturating_sub(17)),
                Style::default().bg(Color::DarkGray),
            ),
        ]);
        frame.render_widget(Paragraph::new(line), area);
    }

    // -- File browser --

    fn render_file_browser(&self, frame: &mut Frame, area: Rect, fb: &FileBrowser) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2), // Header (path + separator)
                Constraint::Min(1),   // File list
            ])
            .split(area);

        // Header: current directory
        let dir_display = fb.current_dir.to_string_lossy();
        let header_lines = vec![
            Line::from(vec![
                Span::styled("  ", Style::default()),
                Span::styled(
                    dir_display.to_string(),
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(Span::styled(
                "─".repeat(area.width as usize),
                Style::default().fg(Color::DarkGray),
            )),
        ];
        frame.render_widget(Paragraph::new(header_lines), chunks[0]);

        // File list
        let list_height = chunks[1].height as usize;
        let mut lines = Vec::new();

        if fb.visible_count() == 0 {
            lines.push(Line::from(Span::styled(
                "  (empty directory)",
                Style::default().fg(Color::DarkGray),
            )));
        } else {
            for i in 0..list_height {
                let idx = fb.scroll_offset + i;
                if idx >= fb.visible_count() {
                    lines.push(Line::from(""));
                    continue;
                }

                let entry_idx = fb.filtered_indices[idx];
                let entry = &fb.entries[entry_idx];
                let is_selected = idx == fb.selected;

                let icon = " ";
                let suffix = if entry.is_dir { "/" } else { "" };

                let name_style = if is_selected {
                    if entry.is_dir {
                        Style::default()
                            .fg(Color::Cyan)
                            .bg(Color::DarkGray)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default()
                            .fg(Color::White)
                            .bg(Color::DarkGray)
                            .add_modifier(Modifier::BOLD)
                    }
                } else if entry.is_dir {
                    Style::default().fg(Color::Cyan)
                } else {
                    Style::default().fg(Color::White)
                };

                let indicator = if is_selected { ">" } else { " " };
                let ind_style = if is_selected {
                    Style::default()
                        .fg(Color::Yellow)
                        .bg(Color::DarkGray)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };

                let size_str = if entry.is_dir {
                    "    dir".to_string()
                } else {
                    format!("{:>7}", format_size(entry.size))
                };

                let name_len = entry.name.len() + suffix.len() + 4; // icon + spaces
                let pad =
                    (area.width as usize).saturating_sub(name_len + size_str.len() + 2);

                let bg = if is_selected {
                    Style::default().bg(Color::DarkGray)
                } else {
                    Style::default()
                };

                lines.push(Line::from(vec![
                    Span::styled(format!(" {} ", indicator), ind_style),
                    Span::styled(icon.to_string(), name_style),
                    Span::styled(format!("{}{}", entry.name, suffix), name_style),
                    Span::styled(" ".repeat(pad), bg),
                    Span::styled(
                        size_str,
                        if is_selected {
                            Style::default().fg(Color::DarkGray).bg(Color::DarkGray)
                        } else {
                            Style::default().fg(Color::DarkGray)
                        },
                    ),
                    Span::styled(" ", bg),
                ]));
            }
        }

        frame.render_widget(Paragraph::new(lines), chunks[1]);
    }

    fn render_file_browser_status(&self, frame: &mut Frame, area: Rect, fb: &FileBrowser) {
        let mode_str = match fb.input_mode {
            BrowserInputMode::Navigate => " FILES ",
            BrowserInputMode::Filter => " FILTER ",
            BrowserInputMode::NewFile => " NEW FILE ",
        };

        let count = format!(
            " {}/{} ",
            if fb.visible_count() > 0 {
                fb.selected + 1
            } else {
                0
            },
            fb.visible_count()
        );

        let mode_span = Span::styled(
            mode_str,
            Style::default()
                .fg(Color::Black)
                .bg(Color::Magenta)
                .add_modifier(Modifier::BOLD),
        );
        let count_span = Span::styled(
            &count,
            Style::default().fg(Color::White).bg(Color::DarkGray),
        );

        let hints = " j/k:nav  enter:open  h/-:up  /:filter  n:new  ~:home  q:close ";
        let used = mode_span.width() + count_span.width() + hints.len();
        let pad = (area.width as usize).saturating_sub(used);

        let line = Line::from(vec![
            mode_span,
            count_span,
            Span::styled(
                " ".repeat(pad),
                Style::default().bg(Color::DarkGray),
            ),
            Span::styled(
                hints,
                Style::default().fg(Color::DarkGray).bg(Color::DarkGray),
            ),
        ]);
        frame.render_widget(Paragraph::new(line), area);
    }

    // -- Tab bar --

    fn render_tab_bar(
        &self,
        frame: &mut Frame,
        area: Rect,
        buffers: &[Buffer],
        active_idx: usize,
    ) {
        let titles: Vec<Line> = buffers
            .iter()
            .enumerate()
            .map(|(i, buf)| {
                let name = if buf.modified {
                    format!(" {} [+] ", buf.name)
                } else {
                    format!(" {} ", buf.name)
                };
                let style = if i == active_idx {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::DarkGray)
                };
                Line::from(Span::styled(name, style))
            })
            .collect();

        let tabs = Tabs::new(titles)
            .style(Style::default().bg(Color::DarkGray))
            .highlight_style(
                Style::default()
                    .fg(Color::Yellow)
                    .bg(Color::Black)
                    .add_modifier(Modifier::BOLD),
            )
            .divider(Span::raw("│"));

        frame.render_widget(tabs, area);
    }

    // -- Editor content --

    fn render_editor(
        &self,
        frame: &mut Frame,
        area: Rect,
        buf: &Buffer,
        cursor: &Cursor,
        scroll_offset: usize,
        is_active: bool,
        highlight_cache: &HighlightCache,
        show_cursor: bool,
        search_state: Option<&SearchState>,
        visual_anchor: Option<(usize, usize)>,
        substitute_highlight: Option<(usize, usize, usize)>,
    ) {
        let editor_height = area.height as usize;
        let line_num_width = buf.line_count().to_string().len().max(3);
        let gutter_width = line_num_width + 2;

        let editor_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(gutter_width as u16),
                Constraint::Min(1),
            ])
            .split(area);

        // Line numbers
        let mut gutter_lines = Vec::new();
        for i in 0..editor_height {
            let line_num = scroll_offset + i;
            if line_num < buf.line_count() {
                let num_str = format!("{:>width$} ", line_num + 1, width = line_num_width);
                let style = if line_num == cursor.line {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::DarkGray)
                };
                gutter_lines.push(Line::from(Span::styled(num_str, style)));
            } else {
                let tilde = format!("{:>width$} ", "~", width = line_num_width);
                gutter_lines.push(Line::from(Span::styled(
                    tilde,
                    Style::default().fg(Color::DarkGray),
                )));
            }
        }
        frame.render_widget(Paragraph::new(gutter_lines), editor_chunks[0]);

        // Collect search matches for highlighting from per-buffer state.
        let buf_search = search_state.and_then(|s| s.buffer_matches.get(&buf.id));
        let search_matches: Vec<&(usize, usize, usize)> = buf_search
            .map(|bm| &bm.matches)
            .into_iter()
            .flatten()
            .collect();
        let current_match_idx = buf_search.and_then(|bm| bm.current_match);

        // Bracket match: find matching bracket for cursor position.
        let bracket_match = find_matching_bracket(buf, cursor.line, cursor.col);

        // Editor content
        let cached = highlight_cache.get(buf.id);
        let mut editor_lines = Vec::new();

        for i in 0..editor_height {
            let line_idx = scroll_offset + i;
            if line_idx >= buf.rope.len_lines() {
                editor_lines.push(Line::from(vec![]));
                continue;
            }

            let mut spans: Vec<Span> = if let Some(highlights) = cached {
                if let Some(segments) = highlights.get(line_idx) {
                    segments
                        .iter()
                        .map(|seg| {
                            Span::styled(seg.text.clone(), Style::default().fg(seg.fg))
                        })
                        .collect()
                } else {
                    let line = buf.rope.line(line_idx);
                    let text: String = line.chars().filter(|c| *c != '\n' && *c != '\r').collect();
                    vec![Span::styled(text, Style::default().fg(Color::White))]
                }
            } else {
                let line = buf.rope.line(line_idx);
                let text: String = line.chars().filter(|c| *c != '\n' && *c != '\r').collect();
                vec![Span::styled(text, Style::default().fg(Color::White))]
            };

            // Overlay search highlights on this line.
            let line_matches: Vec<(usize, usize, bool)> = search_matches
                .iter()
                .enumerate()
                .filter(|(_, m)| m.0 == line_idx)
                .map(|(global_idx, m)| (m.1, m.2, current_match_idx == Some(global_idx)))
                .collect();

            if !line_matches.is_empty() {
                spans = overlay_search_highlights(spans, &line_matches);
            }

            // Overlay visual selection highlights on this line.
            if let Some((anchor_line, anchor_col)) = visual_anchor {
                let sel = visual_line_range(line_idx, anchor_line, anchor_col, cursor.line, cursor.col, buf);
                if let Some((start_col, end_col)) = sel {
                    spans = overlay_visual_highlights(spans, start_col, end_col);
                }
            }

            // Overlay bracket match highlight on this line.
            if let Some((match_line, match_col)) = bracket_match {
                if line_idx == match_line {
                    spans = overlay_bracket_highlight(spans, match_col);
                }
            }

            // Overlay substitute confirm highlight on this line.
            if let Some((sub_line, sub_start, sub_end)) = substitute_highlight {
                if line_idx == sub_line {
                    spans = overlay_search_highlights(
                        spans,
                        &[(sub_start, sub_end, true)],
                    );
                }
            }

            editor_lines.push(Line::from(spans));
        }

        frame.render_widget(Paragraph::new(editor_lines), editor_chunks[1]);

        // Cursor — only for active pane when no overlay is covering it
        if is_active && show_cursor {
            let cursor_x = editor_chunks[1].x + cursor.col as u16;
            let cursor_y = editor_chunks[1].y + (cursor.line.saturating_sub(scroll_offset)) as u16;
            if cursor_x < editor_chunks[1].x + editor_chunks[1].width
                && cursor_y < editor_chunks[1].y + editor_chunks[1].height
            {
                frame.set_cursor_position((cursor_x, cursor_y));
            }
        }
    }

    // -- Status line --

    fn render_status_line(
        &self,
        frame: &mut Frame,
        area: Rect,
        buf: &Buffer,
        cursor: &Cursor,
        mode: &Mode,
    ) {
        let mode_style = match mode {
            Mode::Normal => Style::default().fg(Color::Black).bg(Color::Blue),
            Mode::Insert => Style::default().fg(Color::Black).bg(Color::Green),
            Mode::Visual => Style::default().fg(Color::Black).bg(Color::Magenta),
            Mode::Command | Mode::Search => Style::default().fg(Color::Black).bg(Color::Yellow),
        };

        let mode_span = Span::styled(
            format!(" {} ", mode.display_name()),
            mode_style.add_modifier(Modifier::BOLD),
        );

        let file_name = buf
            .path
            .as_ref()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| buf.name.clone());

        let modified = if buf.modified { " [+]" } else { "" };

        let file_span = Span::styled(
            format!(" {}{} ", file_name, modified),
            Style::default().fg(Color::White).bg(Color::DarkGray),
        );

        let pos_span = Span::styled(
            format!(" {}:{} ", cursor.line + 1, cursor.col + 1),
            Style::default().fg(Color::White).bg(Color::DarkGray),
        );

        let used = mode_span.width() + file_span.width() + pos_span.width();
        let fill = " ".repeat((area.width as usize).saturating_sub(used));
        let fill_span = Span::styled(fill, Style::default().bg(Color::DarkGray));

        let line = Line::from(vec![mode_span, file_span, fill_span, pos_span]);
        frame.render_widget(Paragraph::new(line), area);
    }

    // -- Command line --

    fn render_command_line(
        &self,
        frame: &mut Frame,
        area: Rect,
        mode: &Mode,
        command_buffer: &str,
        status_message: &str,
        pending_keys: &str,
        pending_hints: &[(String, String)],
        file_browser: Option<&FileBrowser>,
        search_buffer: &str,
        search_direction: SearchDirection,
    ) {
        // File browser input modes
        if let Some(fb) = file_browser {
            match fb.input_mode {
                BrowserInputMode::Filter => {
                    let line = Line::from(vec![
                        Span::styled("/", Style::default().fg(Color::Yellow)),
                        Span::styled(
                            fb.filter.clone(),
                            Style::default().fg(Color::White),
                        ),
                    ]);
                    frame.render_widget(Paragraph::new(line), area);
                    return;
                }
                BrowserInputMode::NewFile => {
                    let line = Line::from(vec![
                        Span::styled(
                            "New file: ",
                            Style::default().fg(Color::Yellow),
                        ),
                        Span::styled(
                            fb.new_file_name.clone(),
                            Style::default().fg(Color::White),
                        ),
                    ]);
                    frame.render_widget(Paragraph::new(line), area);
                    return;
                }
                _ => {}
            }
        }

        let content = if *mode == Mode::Search {
            let prefix = match search_direction {
                SearchDirection::Forward => "/",
                SearchDirection::Backward => "?",
            };
            Line::from(vec![
                Span::styled(prefix, Style::default().fg(Color::Yellow)),
                Span::styled(search_buffer.to_string(), Style::default().fg(Color::White)),
            ])
        } else if *mode == Mode::Command {
            Line::from(vec![
                Span::styled(":", Style::default().fg(Color::White)),
                Span::styled(command_buffer.to_string(), Style::default().fg(Color::White)),
            ])
        } else if !pending_keys.is_empty() {
            // Show which-key hints: pending keys followed by available next keys.
            let mut spans = vec![
                Span::styled(
                    format!("{}-  ", pending_keys),
                    Style::default().fg(Color::Cyan),
                ),
            ];
            let max_width = area.width as usize;
            let mut used = pending_keys.len() + 3;
            for (key, desc) in pending_hints {
                let hint = format!("{}:{} ", key, desc);
                if used + hint.len() > max_width {
                    break;
                }
                spans.push(Span::styled(
                    key.clone(),
                    Style::default().fg(Color::Yellow),
                ));
                spans.push(Span::styled(
                    format!(":{} ", desc),
                    Style::default().fg(Color::DarkGray),
                ));
                used += hint.len();
            }
            Line::from(spans)
        } else if !status_message.is_empty() {
            Line::from(Span::styled(
                status_message.to_string(),
                Style::default().fg(Color::Yellow),
            ))
        } else {
            Line::from(Span::styled(
                "SPC for leader | : for commands | SPC q q to quit",
                Style::default().fg(Color::DarkGray),
            ))
        };

        frame.render_widget(Paragraph::new(content), area);
    }

    fn render_find_file(&self, frame: &mut Frame, area: Rect, ff: &FindFileState) {
        // Centered popup, same dimensions as palette.
        let width = (area.width * 3 / 5).max(40).min(area.width.saturating_sub(4));
        let max_height = (area.height * 7 / 10).max(10).min(area.height.saturating_sub(2));
        let x = (area.width.saturating_sub(width)) / 2;
        let y = (area.height.saturating_sub(max_height)) / 2;
        let popup_area = Rect::new(x, y, width, max_height);

        frame.render_widget(Clear, popup_area);

        let block = Block::default()
            .borders(Borders::ALL)
            .title(" Find File ")
            .border_style(Style::default().fg(Color::Cyan));

        let inner = block.inner(popup_area);
        frame.render_widget(block, popup_area);

        if inner.height < 2 || inner.width < 4 {
            return;
        }

        // Path input line.
        let input_area = Rect::new(inner.x, inner.y, inner.width, 1);
        let display_path = ff.display_path();
        // Show the tail that fits in the width, so the cursor end is always visible.
        let visible_len = inner.width.saturating_sub(2) as usize;
        let path_display = if display_path.len() > visible_len {
            &display_path[display_path.len() - visible_len..]
        } else {
            &display_path
        };
        let input_line = Line::from(vec![
            Span::styled("> ", Style::default().fg(Color::Cyan)),
            Span::raw(path_display),
        ]);
        frame.render_widget(Paragraph::new(input_line), input_area);

        // Completions list.
        let list_area = Rect::new(inner.x, inner.y + 1, inner.width, inner.height.saturating_sub(1));
        let visible_count = list_area.height as usize;

        let scroll_offset = if ff.selected >= visible_count {
            ff.selected - visible_count + 1
        } else {
            0
        };

        let mut lines = Vec::new();
        for (i, &entry_idx) in ff.filtered.iter().skip(scroll_offset).take(visible_count).enumerate() {
            let entry = &ff.entries[entry_idx];
            let is_selected = i + scroll_offset == ff.selected;
            let style = if is_selected {
                Style::default().bg(Color::DarkGray).fg(Color::White)
            } else {
                Style::default()
            };

            let icon = if entry.is_dir { "/" } else { "" };
            let name = format!("{}{}", entry.name, icon);

            // Pad to full width.
            let available = list_area.width as usize;
            let padded = if name.len() < available {
                format!("{}{}", name, " ".repeat(available - name.len()))
            } else {
                name[..available].to_string()
            };

            lines.push(Line::from(Span::styled(padded, style)));
        }

        // If no matches, show a "create" hint.
        if ff.filtered.is_empty() && !ff.input.is_empty() {
            lines.push(Line::from(Span::styled(
                format!("  Press Enter to create \"{}\"", ff.input),
                Style::default().fg(Color::Yellow).add_modifier(Modifier::ITALIC),
            )));
        }

        frame.render_widget(Paragraph::new(lines), list_area);
    }

    fn render_palette(&self, frame: &mut Frame, area: Rect, palette: &PaletteState) {
        // Centered popup: 60% width, up to 70% height
        let width = (area.width * 3 / 5).max(40).min(area.width.saturating_sub(4));
        let max_height = (area.height * 7 / 10).max(10).min(area.height.saturating_sub(2));
        let x = (area.width.saturating_sub(width)) / 2;
        let y = (area.height.saturating_sub(max_height)) / 2;
        let popup_area = Rect::new(x, y, width, max_height);

        frame.render_widget(Clear, popup_area);

        let block = Block::default()
            .borders(Borders::ALL)
            .title(" Actions (type to filter) ")
            .border_style(Style::default().fg(Color::Cyan));

        let inner = block.inner(popup_area);
        frame.render_widget(block, popup_area);

        if inner.height < 2 || inner.width < 4 {
            return;
        }

        // Input line
        let input_area = Rect::new(inner.x, inner.y, inner.width, 1);
        let input_line = Line::from(vec![
            Span::styled("> ", Style::default().fg(Color::Cyan)),
            Span::raw(&palette.query),
        ]);
        frame.render_widget(Paragraph::new(input_line), input_area);

        // Results
        let list_area = Rect::new(
            inner.x,
            inner.y + 1,
            inner.width,
            inner.height.saturating_sub(1),
        );
        let visible_count = list_area.height as usize;

        // Scroll to keep selected item visible
        let scroll_offset = if palette.selected >= visible_count {
            palette.selected - visible_count + 1
        } else {
            0
        };

        let mut lines = Vec::new();
        for (i, &item_idx) in palette
            .filtered
            .iter()
            .skip(scroll_offset)
            .take(visible_count)
            .enumerate()
        {
            let item = &palette.items[item_idx];
            let is_selected = i + scroll_offset == palette.selected;
            let style = if is_selected {
                Style::default().bg(Color::DarkGray).fg(Color::White)
            } else {
                Style::default()
            };

            // Compute padding for right-aligned binding
            let name_len = item.name.len();
            let binding_len = item.binding.len();
            let available = list_area.width as usize;
            let gap = if name_len + binding_len + 2 < available {
                available - name_len - binding_len
            } else {
                2
            };

            let line = Line::from(vec![
                Span::styled(&item.name, style),
                Span::styled(" ".repeat(gap), style),
                Span::styled(
                    &item.binding,
                    style.add_modifier(Modifier::DIM),
                ),
            ]);
            lines.push(line);
        }

        frame.render_widget(Paragraph::new(lines), list_area);
    }
}

/// Overlay search match highlights on a line's existing spans.
/// `matches` is a list of (col_start, col_end, is_current) for matches on this line.
fn overlay_search_highlights<'a>(
    spans: Vec<Span<'a>>,
    matches: &[(usize, usize, bool)],
) -> Vec<Span<'a>> {
    let highlight_style = Style::default().fg(Color::Black).bg(Color::Yellow);
    let current_style = Style::default().fg(Color::Black).bg(Color::Rgb(255, 150, 50));

    let mut result = Vec::new();
    let mut col = 0usize;

    for span in spans {
        let span_text: Vec<char> = span.content.chars().collect();
        let span_start = col;
        let span_end = col + span_text.len();
        let base_style = span.style;

        let mut pos = 0; // Position within this span (in chars)

        for &(m_start, m_end, is_current) in matches {
            // Clip match to this span's range.
            let hl_start = m_start.max(span_start);
            let hl_end = m_end.min(span_end);

            if hl_start >= hl_end || hl_start >= span_end || hl_end <= span_start {
                continue;
            }

            let local_start = (hl_start - span_start).max(pos);
            let local_end = hl_end - span_start;

            if local_start >= local_end {
                continue;
            }

            // Emit pre-match text with original style.
            if pos < local_start {
                let text: String = span_text[pos..local_start].iter().collect();
                result.push(Span::styled(text, base_style));
            }

            // Emit match text with highlight style.
            let text: String = span_text[local_start..local_end].iter().collect();
            let style = if is_current { current_style } else { highlight_style };
            result.push(Span::styled(text, style));

            pos = local_end;
        }

        // Emit remaining text after last match.
        if pos < span_text.len() {
            let text: String = span_text[pos..].iter().collect();
            result.push(Span::styled(text, base_style));
        }

        col = span_end;
    }

    result
}

/// Compute the column range to highlight on a given line for visual selection.
/// Returns `Some((start_col, end_col))` (exclusive end) if this line is within the selection.
fn visual_line_range(
    line_idx: usize,
    anchor_line: usize,
    anchor_col: usize,
    cursor_line: usize,
    cursor_col: usize,
    buf: &Buffer,
) -> Option<(usize, usize)> {
    // Normalize so start <= end
    let (start_line, start_col, end_line, end_col) = if (anchor_line, anchor_col) <= (cursor_line, cursor_col) {
        (anchor_line, anchor_col, cursor_line, cursor_col)
    } else {
        (cursor_line, cursor_col, anchor_line, anchor_col)
    };

    if line_idx < start_line || line_idx > end_line {
        return None;
    }

    let line_len = buf.rope.line(line_idx).chars().filter(|c| *c != '\n' && *c != '\r').count();

    let col_start = if line_idx == start_line { start_col } else { 0 };
    let col_end = if line_idx == end_line { (end_col + 1).min(line_len) } else { line_len };

    if col_start >= col_end {
        return None;
    }

    Some((col_start, col_end))
}

/// Overlay visual selection highlighting on a line's spans.
fn overlay_visual_highlights<'a>(spans: Vec<Span<'a>>, sel_start: usize, sel_end: usize) -> Vec<Span<'a>> {
    let visual_style = Style::default().fg(Color::White).bg(Color::Rgb(68, 68, 120));

    let mut result = Vec::new();
    let mut col = 0usize;

    for span in spans {
        let span_chars: Vec<char> = span.content.chars().collect();
        let span_start = col;
        let span_end = col + span_chars.len();
        let base_style = span.style;

        // Clip selection to this span
        let hl_start = sel_start.max(span_start);
        let hl_end = sel_end.min(span_end);

        if hl_start >= hl_end || hl_start >= span_end || hl_end <= span_start {
            // No overlap — emit span as-is
            result.push(span);
            col = span_end;
            continue;
        }

        let local_start = hl_start - span_start;
        let local_end = hl_end - span_start;

        // Pre-selection
        if local_start > 0 {
            let text: String = span_chars[..local_start].iter().collect();
            result.push(Span::styled(text, base_style));
        }

        // Selected
        let text: String = span_chars[local_start..local_end].iter().collect();
        result.push(Span::styled(text, visual_style));

        // Post-selection
        if local_end < span_chars.len() {
            let text: String = span_chars[local_end..].iter().collect();
            result.push(Span::styled(text, base_style));
        }

        col = span_end;
    }

    result
}

/// Return the matching bracket char for an opening/closing bracket.
fn bracket_pair(c: char) -> Option<(char, bool)> {
    match c {
        '(' => Some((')', true)),   // opening, scan forward
        ')' => Some(('(', false)),
        '[' => Some((']', true)),
        ']' => Some(('[', false)),
        '{' => Some(('}', true)),
        '}' => Some(('{', false)),
        '<' => Some(('>', true)),
        '>' => Some(('<', false)),
        _ => None,
    }
}

/// Find the matching bracket for the character at (line, col).
/// Returns Some((match_line, match_col)) or None.
fn find_matching_bracket(buf: &Buffer, line: usize, col: usize) -> Option<(usize, usize)> {
    if line >= buf.rope.len_lines() {
        return None;
    }
    let rope_line = buf.rope.line(line);
    if col >= rope_line.len_chars() {
        return None;
    }
    let ch = rope_line.char(col);
    let (target, forward) = bracket_pair(ch)?;

    let mut depth: i32 = 0;
    if forward {
        // Scan forward from (line, col+1)
        let mut l = line;
        let mut c = col + 1;
        let total_lines = buf.rope.len_lines();
        while l < total_lines {
            let rl = buf.rope.line(l);
            let len = rl.len_chars();
            while c < len {
                let rc = rl.char(c);
                if rc == ch {
                    depth += 1;
                } else if rc == target {
                    if depth == 0 {
                        return Some((l, c));
                    }
                    depth -= 1;
                }
                c += 1;
            }
            l += 1;
            c = 0;
        }
    } else {
        // Scan backward from (line, col-1)
        let mut l = line;
        let mut c = col as i64 - 1;
        loop {
            if c < 0 {
                if l == 0 {
                    break;
                }
                l -= 1;
                c = buf.rope.line(l).len_chars() as i64 - 1;
                continue;
            }
            let rc = buf.rope.line(l).char(c as usize);
            if rc == ch {
                depth += 1;
            } else if rc == target {
                if depth == 0 {
                    return Some((l, c as usize));
                }
                depth -= 1;
            }
            c -= 1;
        }
    }
    None
}

/// Highlight a single character at `col` with the bracket match style.
fn overlay_bracket_highlight<'a>(spans: Vec<Span<'a>>, match_col: usize) -> Vec<Span<'a>> {
    let style = Style::default()
        .fg(Color::Rgb(255, 215, 0))
        .add_modifier(Modifier::BOLD);

    let mut result = Vec::new();
    let mut col = 0usize;

    for span in spans {
        let span_chars: Vec<char> = span.content.chars().collect();
        let span_start = col;
        let span_end = col + span_chars.len();
        let base_style = span.style;

        if match_col >= span_start && match_col < span_end {
            let local = match_col - span_start;
            if local > 0 {
                let text: String = span_chars[..local].iter().collect();
                result.push(Span::styled(text, base_style));
            }
            let text: String = span_chars[local..local + 1].iter().collect();
            result.push(Span::styled(text, style));
            if local + 1 < span_chars.len() {
                let text: String = span_chars[local + 1..].iter().collect();
                result.push(Span::styled(text, base_style));
            }
        } else {
            result.push(span);
        }

        col = span_end;
    }

    result
}
