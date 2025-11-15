pub mod import {
    pub use color_eyre::eyre;
    pub use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
    pub use immutable_map::TreeMap;
    pub use ratatui::{
        buffer::Buffer,
        layout::Rect,
        style::Stylize,
        symbols::border,
        text::{Line, Text},
        widgets::{Block, Paragraph, Widget},
        DefaultTerminal, Frame,
    };
    pub use snafu::prelude::*;
    pub use std::{
        collections::HashSet, error, fmt, hash::Hash, io, iter::Iterator, rc::Rc, slice::Iter,
    };

    pub use crate::app;
    pub use crate::command;
    pub use crate::error::*;
    pub use crate::prefix_tree;
}

pub mod app;
pub mod command;
pub mod error;
pub mod prefix_tree;
