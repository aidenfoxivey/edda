// https://docs.rs/meshtastic/latest/meshtastic/
// https://docs.rs/sqlite/latest/sqlite/
// https://docs.rs/ratatui/latest/ratatui/
//
// A few goals for the project:
// - graceful degradation on disconnection
// - clear UI for sending messages
// - support direct messages

use std::collections::HashMap;
use std::time::{Duration, Instant, SystemTime};

use color_eyre::Result;

use meshtastic::protobufs::{NodeInfo};
use meshtastic::types::NodeId;

use ratatui::DefaultTerminal;
use ratatui::{
    crossterm::event::{self, Event, KeyCode},
    prelude::*,
    widgets::{
        Block, List, ListState, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap,
    },
};
use tokio::sync::mpsc;

mod mesh;

#[derive(Debug)]
struct Message {
    to: NodeId,
    name: String,
    ts: SystemTime,
}

struct App {
    pub vertical_scroll_state: ScrollbarState,
    pub horizontal_scroll_state: ScrollbarState,
    pub vertical_scroll: usize,
    pub horizontal_scroll: usize,
    pub nodes: HashMap<u32, NodeInfo>,
    pub input: String,
    pub focus: Option<Focus>,
    pub node_list_state: ListState,
    pub current_contact: Option<NodeInfo>,
    pub state: AppState,
    // pub current_conversation: Vec<Message>,
}

impl Default for App {
    fn default() -> Self {
        Self {
            vertical_scroll_state: ScrollbarState::default(),
            horizontal_scroll_state: ScrollbarState::default(),
            vertical_scroll: 0,
            horizontal_scroll: 0,
            nodes: HashMap::new(),
            input: String::new(),
            focus: None,
            node_list_state: ListState::default(),
            current_contact: None,
            state: AppState::Loading,
        }
    }
}

#[derive(PartialEq)]
enum AppState {
    Loading,
    Loaded,
}

/// The specific element of the UI that is currently focused.
#[derive(PartialEq, Copy, Clone)]
enum Focus {
    NodeList,
    Conversation,
    Input,
}

fn main() -> Result<()> {
    color_eyre::install()?;
    let (tx, rx) = mpsc::channel(100);

    // Run a seperate thread that listens to the Meshtastic interface.
    std::thread::spawn(move || {
        if let Err(e) = mesh::run_meshtastic(tx) {
            eprintln!("Meshtastic thread error: {}", e);
        }
    });

    // Generate the terminal handlers and run the Ratatui application.
    let mut terminal = ratatui::init();
    let mut app = App::default();
    // Take a receiver to transport information between the Meshtastic thread and the terminal thread.
    let app_result = app.run(&mut terminal, rx);
    ratatui::restore();
    app_result
}

impl App {
    fn get_sorted_nodes(&self) -> Vec<&NodeInfo> {
        let mut nodes: Vec<_> = self.nodes.values().collect();
        nodes.sort_by_key(|n| n.num);
        nodes
    }

    fn next_node(&mut self) {
        if self.nodes.is_empty() {
            return;
        }
        let i = match self.node_list_state.selected() {
            Some(i) => {
                if i < self.nodes.len() - 1 {
                    i + 1
                } else {
                    i
                }
            }
            None => 0,
        };
        self.node_list_state.select(Some(i));
    }

    fn previous_node(&mut self) {
        if self.nodes.is_empty() {
            return;
        }
        let i = match self.node_list_state.selected() {
            Some(i) => {
                if i > 0 {
                    i - 1
                } else {
                    i
                }
            }
            None => 0,
        };
        self.node_list_state.select(Some(i));
    }

    fn run(
        &mut self,
        terminal: &mut DefaultTerminal,
        mut rx: mpsc::Receiver<NodeInfo>,
    ) -> Result<()> {
        let tick_rate = Duration::from_millis(250);
        let mut last_tick = Instant::now();
        loop {
            terminal.draw(|frame| self.draw(frame))?;

            if let Ok(node_info) = rx.try_recv() {
                let is_empty = self.nodes.is_empty();
                self.nodes.insert(node_info.num, node_info);
                if is_empty {
                    self.node_list_state.select(Some(0));
                }
                self.state = AppState::Loaded;
            }

            let timeout = tick_rate.saturating_sub(last_tick.elapsed());
            if event::poll(timeout)?
                && let Event::Key(key) = event::read()? {
                    if self.state == AppState::Loading {
                        if let KeyCode::Char('q') = key.code {
                            return Ok(());
                        }
                    } else {
                        match key.code {
                            KeyCode::Esc => {
                                self.focus = None;
                            }
                            KeyCode::Tab => {
                                self.focus = match self.focus {
                                    Some(Focus::NodeList) => Some(Focus::Conversation),
                                    Some(Focus::Conversation) => Some(Focus::Input),
                                    Some(Focus::Input) => Some(Focus::NodeList),
                                    None => Some(Focus::NodeList),
                                };
                            }
                            _ => {
                                if let Some(focus) = self.focus {
                                    match focus {
                                        Focus::NodeList => match key.code {
                                            KeyCode::Char('j') | KeyCode::Down => self.next_node(),
                                            KeyCode::Char('k') | KeyCode::Up => {
                                                self.previous_node()
                                            }
                                            KeyCode::Enter => {
                                                if let Some(selected_index) =
                                                    self.node_list_state.selected()
                                                {
                                                    let nodes = self.get_sorted_nodes();
                                                    if let Some(selected_node) =
                                                        nodes.get(selected_index)
                                                    {
                                                        self.current_contact =
                                                            Some((*selected_node).clone());
                                                    }
                                                }
                                            }
                                            _ => {}
                                        },
                                        Focus::Conversation => match key.code {
                                            KeyCode::Char('j') | KeyCode::Down => {
                                                self.vertical_scroll =
                                                    self.vertical_scroll.saturating_add(1);
                                                self.vertical_scroll_state = self
                                                    .vertical_scroll_state
                                                    .position(self.vertical_scroll);
                                            }
                                            KeyCode::Char('k') | KeyCode::Up => {
                                                self.vertical_scroll =
                                                    self.vertical_scroll.saturating_sub(1);
                                                self.vertical_scroll_state = self
                                                    .vertical_scroll_state
                                                    .position(self.vertical_scroll);
                                            }
                                            KeyCode::Char('h') | KeyCode::Left => {
                                                self.horizontal_scroll =
                                                    self.horizontal_scroll.saturating_sub(1);
                                                self.horizontal_scroll_state = self
                                                    .horizontal_scroll_state
                                                    .position(self.horizontal_scroll);
                                            }
                                            KeyCode::Char('l') | KeyCode::Right => {
                                                self.horizontal_scroll =
                                                    self.horizontal_scroll.saturating_add(1);
                                                self.horizontal_scroll_state = self
                                                    .horizontal_scroll_state
                                                    .position(self.horizontal_scroll);
                                            }
                                            _ => {}
                                        },
                                        Focus::Input => match key.code {
                                            KeyCode::Char(c) => {
                                                self.input.push(c);
                                            }
                                            KeyCode::Backspace => {
                                                self.input.pop();
                                            }
                                            KeyCode::Enter => {
                                                self.input.push('\n');
                                            }
                                            _ => {}
                                        },
                                    }
                                } else if let KeyCode::Char('q') = key.code {
                                    return Ok(());
                                }
                            }
                        }
                    }
                }
            if last_tick.elapsed() >= tick_rate {
                last_tick = Instant::now();
            }
        }
    }

    fn draw(&mut self, frame: &mut Frame) {
        if self.state == AppState::Loading {
            self.draw_loading(frame);
            return;
        }

        let area = frame.area();

        let s =
            "Veeeeeeeeeeeeeeeery    loooooooooooooooooong   striiiiiiiiiiiiiiiiiiiiiiiiiing.   ";
        let mut long_line = s.repeat(usize::from(area.width) / s.len() + 4);
        long_line.push('\n');

        let horizontal_chunks =
            Layout::horizontal([Constraint::Percentage(30), Constraint::Percentage(70)])
                .split(area);

        let chunks = Layout::vertical([
            Constraint::Min(1),
            Constraint::Percentage(10),
            Constraint::Percentage(90),
        ])
        .split(horizontal_chunks[1]);

        let text = vec![
            Line::from("This is a line "),
            Line::from("This is a line   ".red()),
            Line::from("This is a line".on_dark_gray()),
            Line::from("This is a longer line".crossed_out()),
            Line::from(long_line.clone()),
            Line::from("This is a line".reset()),
            Line::from(vec![
                "Masked text: ".into(),
                Span::styled(Masked::new("password", '*'), Style::new().fg(Color::Red)),
            ]),
        ];
        self.vertical_scroll_state = self.vertical_scroll_state.content_length(text.len());
        self.horizontal_scroll_state = self.horizontal_scroll_state.content_length(long_line.len());

        let title = Block::new()
            .title_alignment(Alignment::Center)
            .title("MESHCOM 0.0.1".bold());
        frame.render_widget(title, chunks[0]);

        let title = if let Some(contact) = &self.current_contact {
            format!("CONNECTED: {}", contact.user.as_ref().unwrap().long_name)
        } else {
            "NO NODE CONNECTED".to_string()
        };

        let paragraph = Paragraph::new(text.clone())
            .gray()
            .block(
                Block::bordered()
                    .gray()
                    .title(title.as_str().bold())
                    .border_style(if self.focus == Some(Focus::Conversation) {
                        Style::default().fg(Color::Yellow)
                    } else {
                        Style::default()
                    }),
            )
            .scroll((self.vertical_scroll as u16, 0));
        frame.render_widget(paragraph, chunks[2]);
        frame.render_stateful_widget(
            Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("#"))
                .end_symbol(Some("#")),
            chunks[1],
            &mut self.vertical_scroll_state,
        );

        let nodes_list_block = Block::bordered()
            .gray()
            .title("NODE LIST".bold())
            .border_style(if self.focus == Some(Focus::NodeList) {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default()
            });

        let sorted_nodes = self.get_sorted_nodes();
        let items: Vec<_> = sorted_nodes
            .iter()
            .map(|nodeinfo| {
                let long_name = if let Some(user) = nodeinfo.user.as_ref() {
                    user.long_name.clone()
                } else {
                    String::from("UNK")
                };
                let mut line = Line::from(long_name);
                if self.current_contact == Some((*nodeinfo).clone()) {
                    line = line.patch_style(
                        Style::default()
                            .add_modifier(Modifier::BOLD)
                            .fg(Color::Cyan),
                    );
                }
                line
            })
            .collect();

        let list = List::new(items)
            .block(nodes_list_block)
            .highlight_symbol("> ")
            .highlight_style(Style::default().bg(Color::DarkGray));

        frame.render_stateful_widget(list, horizontal_chunks[0], &mut self.node_list_state);

        let input_box = Paragraph::new(self.input.as_str())
            .block(Block::bordered().title("INPUT".bold()).border_style(
                if self.focus == Some(Focus::Input) {
                    Style::default().fg(Color::Yellow)
                } else {
                    Style::default()
                },
            ))
            .wrap(Wrap { trim: false });
        frame.render_widget(input_box, chunks[1]);

        if self.focus == Some(Focus::Input) {
            let input_width = chunks[1].width.saturating_sub(2); // Subtract 2 for borders
            let line_count = (self.input.len() as u16 / input_width) + 1;
            let cursor_x = chunks[1].x + (self.input.len() as u16 % input_width) + 1;
            let cursor_y = chunks[1].y + line_count;
            frame.set_cursor_position((cursor_x, cursor_y));
        }
    }

    fn draw_loading(&self, frame: &mut Frame) {
        let area = frame.area();
        let loading_text = "Loading...";
        let loading_paragraph = Paragraph::new(loading_text)
            .style(Style::default().fg(Color::White))
            .alignment(Alignment::Center);

        let vertical_chunks = Layout::vertical([
            Constraint::Percentage(50),
            Constraint::Length(1),
            Constraint::Percentage(50),
        ])
        .split(area);

        let horizontal_chunks = Layout::horizontal([
            Constraint::Percentage(50),
            Constraint::Length(loading_text.len() as u16),
            Constraint::Percentage(50),
        ])
        .split(vertical_chunks[1]);

        frame.render_widget(loading_paragraph, horizontal_chunks[1]);
    }
}
