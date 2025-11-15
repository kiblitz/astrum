use crate::import::*;

#[derive(Debug, Default)]
pub struct App {
    counter: u8,
    exit: bool,
}

impl App {
    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        while !self.exit {
            terminal.draw(|frame| self.draw(frame)).context(IoSnafu)?;
            self.handle_events()?;
        }
        Ok(())
    }

    fn draw(&self, frame: &mut Frame) {
        frame.render_widget(self, frame.area())
    }

    fn handle_input(&mut self, input: std::collections::HashSet<KeyCode>) {
        if input.contains(&KeyCode::Left) {
            self.counter -= 1;
        }

        if input.contains(&KeyCode::Right) {
            self.counter += 1;
        }
    }

    fn handle_events(&mut self) -> Result<()> {
        let mut input = std::collections::HashSet::new();
        match event::read().context(IoSnafu)? {
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                match key_event.code {
                    KeyCode::Char('q') => self.exit = true,
                    key_code => _ = input.insert(key_code),
                }
            }
            _ => {}
        };
        self.handle_input(input);
        Ok(())
    }
}

impl Widget for &App {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let title = Line::from(" Counter App Tutorial ".bold());
        let instructions = Line::from(vec![
            " Decrement ".into(),
            "<Left>".blue().bold(),
            " Increment ".into(),
            "<Right>".blue().bold(),
            " Quit ".into(),
            "<Q> ".blue().bold(),
        ]);
        let block = Block::bordered()
            .title(title.centered())
            .title_bottom(instructions.centered())
            .border_set(border::THICK);

        let text = Text::from(vec![Line::from(vec![
            "Value: ".into(),
            self.counter.to_string().yellow(),
        ])]);
        Paragraph::new(text)
            .centered()
            .block(block)
            .render(area, buf);
    }
}
