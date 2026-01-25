//! The UI code as well as business logic.

use std::{collections::HashMap, time::Duration};

use color_eyre::eyre::Result;
use meshtastic::protobufs::NodeInfo;
use ratatui::{
    DefaultTerminal,
    widgets::{ListState, ScrollbarState},
};
use tokio::{sync::mpsc, time::Instant};

use crate::types::{AppState, Focus, MeshEvent, UiEvent};

use ratatui::{
    crossterm::event::{self, Event, KeyCode},
    prelude::*,
    widgets::{Block, List, Paragraph, Scrollbar, ScrollbarOrientation, Wrap},
};

pub struct App {
    pub receiver: mpsc::Receiver<MeshEvent>,
    pub transmitter: mpsc::Sender<UiEvent>,
    pub vertical_scroll_state: ScrollbarState,
    pub nodes: HashMap<u32, NodeInfo>,
    pub input: String,
    pub focus: Option<Focus>,
    pub node_list_state: ListState,
    pub current_contact: Option<NodeInfo>,
    pub state: AppState,
    pub current_conversation: Vec<String>,
}

impl App {
    pub fn new(receiver: mpsc::Receiver<MeshEvent>, transmitter: mpsc::Sender<UiEvent>) -> Self {
        Self {
            receiver,
            transmitter,
            vertical_scroll_state: ScrollbarState::default(),
            nodes: HashMap::new(),
            input: String::new(),
            focus: None,
            node_list_state: ListState::default(),
            current_contact: None,
            state: AppState::Loading,
            current_conversation: vec![],
        }
    }

    fn get_sorted_nodes(&self) -> Vec<&NodeInfo> {
        let mut nodes: Vec<_> = self.nodes.values().collect();
        nodes.sort_by_key(|n| n.num);
        nodes
    }

    fn update(&mut self) {
        while let Ok(event) = self.receiver.try_recv() {
            match event {
                MeshEvent::NodeAvailable(node_info) => {
                    let is_empty = self.nodes.is_empty();
                    self.nodes.insert(node_info.num, *node_info);
                    if is_empty {
                        self.node_list_state.select(Some(0));
                    }
                    self.state = AppState::Loaded;
                }
                MeshEvent::Message { node_id: _, message } => {
                    self.current_conversation.push(message);
                }
            }
        }
        self.state = AppState::Loaded;
    }

    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        let tick_rate = Duration::from_millis(250);
        let mut last_tick = Instant::now();
        loop {
            terminal.draw(|frame| self.draw(frame))?;

            self.update();

            let timeout = tick_rate.saturating_sub(last_tick.elapsed());
            if event::poll(timeout)?
                && let Event::Key(key) = event::read()?
            {
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
                                None => Some(Focus::NodeList),
                                Some(Focus::NodeList) => Some(Focus::Conversation),
                                Some(Focus::Conversation) => Some(Focus::Input),
                                Some(Focus::Input) => Some(Focus::NodeList),
                            };
                        }
                        KeyCode::BackTab => {
                            self.focus = match self.focus {
                                None => Some(Focus::NodeList),
                                Some(Focus::NodeList) => Some(Focus::Input),
                                Some(Focus::Input) => Some(Focus::Conversation),
                                Some(Focus::Conversation) => Some(Focus::NodeList),
                            };
                        }
                        _ => {
                            if let Some(focus) = self.focus {
                                match focus {
                                    Focus::NodeList => match key.code {
                                        KeyCode::Char('j') | KeyCode::Down => {
                                            self.node_list_state.select_next()
                                        }
                                        KeyCode::Char('k') | KeyCode::Up => {
                                            self.node_list_state.select_previous()
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
                                            self.vertical_scroll_state.next();
                                        }
                                        KeyCode::Char('k') | KeyCode::Up => {
                                            self.vertical_scroll_state.prev();
                                        }
                                        _ => {}
                                    },
                                    Focus::Input => match key.code {
                                        KeyCode::Char(c) => {
                                            // Only add character if we're under 237 bytes
                                            if self.input.len() < 237 {
                                                self.input.push(c);
                                            }
                                        }
                                        KeyCode::Backspace => {
                                            self.input.pop();
                                        }
                                        KeyCode::Enter => {
                                            let trimmed = self.input.trim().to_string();
                                            assert!(trimmed.len() <= 237);

                                            if !trimmed.is_empty() {
                                                if let Some(contact) = &self.current_contact {
                                                    if let Ok(_) = self.transmitter.try_send(UiEvent::Message {
                                                        node_id: contact.num.into(),
                                                        message: trimmed.clone(),
                                                    }) {
                                                        self.current_conversation.push(trimmed);
                                                    }
                                                }
                                            }
                                            self.input.clear();
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

        let horizontal_chunks =
            Layout::horizontal([Constraint::Percentage(30), Constraint::Percentage(70)])
                .split(area);

        let chunks = Layout::vertical([
            Constraint::Min(1),
            Constraint::Percentage(10),
            Constraint::Percentage(90),
        ])
        .split(horizontal_chunks[1]);

        let text: Vec<Line> = self.current_conversation.iter().map(|msg| Line::from(msg.as_str())).collect();
        self.vertical_scroll_state = self.vertical_scroll_state.content_length(text.len());

        let title = Block::new()
            .title_alignment(Alignment::Center)
            .title("MESHCOM 0.0.1".bold());
        frame.render_widget(title, chunks[0]);

        let title = if let Some(contact) = &self.current_contact {
            format!("CONNECTED: {}", contact.user.as_ref().unwrap().long_name)
        } else {
            "NO NODE CONNECTED".to_string()
        };

        let paragraph = Paragraph::new(text.clone()).gray().block(
            Block::bordered()
                .gray()
                .title(title.as_str().bold())
                .border_style(if self.focus == Some(Focus::Conversation) {
                    Style::default().fg(Color::Yellow)
                } else {
                    Style::default()
                }),
        );
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
