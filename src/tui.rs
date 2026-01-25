//! The UI code as well as business logic.

use std::{collections::HashMap, time::Duration};

use color_eyre::eyre::Result;
use log::logger;
use meshtastic::protobufs::NodeInfo;
use ratatui::{
    DefaultTerminal,
    widgets::{ListState, ScrollbarState},
};
use tokio::{sync::mpsc, time::Instant};

use crate::types::{Focus, MeshEvent};

use ratatui::{
    crossterm::event::{self, Event, KeyCode},
    prelude::*,
    widgets::{Block, List, Paragraph, Scrollbar, ScrollbarOrientation, Wrap},
};

pub struct App {
    pub receiver: mpsc::Receiver<MeshEvent>,
    pub vertical_scroll_state: ScrollbarState,
    pub horizontal_scroll_state: ScrollbarState,
    pub nodes: HashMap<u32, NodeInfo>,
    pub input: String,
    pub search: String,
    pub focus: Option<Focus>,
    pub node_list_state: ListState,
    pub current_contact: Option<NodeInfo>,
    pub current_conversation: Vec<String>,
}

impl App {
    pub fn new(receiver: mpsc::Receiver<MeshEvent>) -> Self {
        Self {
            receiver,
            vertical_scroll_state: ScrollbarState::default(),
            horizontal_scroll_state: ScrollbarState::default(),
            nodes: HashMap::new(),
            input: String::new(),
            search: String::new(),
            focus: None,
            node_list_state: ListState::default(),
            current_contact: None,
            current_conversation: Vec::new(),
        }
    }

    fn get_sorted_nodes(&self) -> Vec<&NodeInfo> {
        let mut nodes: Vec<_> = self.nodes.values().collect();
        nodes.sort_by_key(|n| n.num);
        nodes
    }

    fn update(&mut self) {
        if let Ok(MeshEvent::NodeAvailable(node_info)) = self.receiver.try_recv() {
            log::info!("Node {:#?} added", node_info);
            let is_empty = self.nodes.is_empty();
            self.nodes.insert(node_info.num, *node_info);
            if is_empty {
                self.node_list_state.select(Some(0));
            }
        }
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
                match key.code {
                    KeyCode::Esc => {
                        self.focus = None;
                    }
                    KeyCode::Tab => {
                        self.focus = match self.focus {
                            None => Some(Focus::NodeList),
                            Some(Focus::NodeList) => Some(Focus::Conversation),
                            Some(Focus::Conversation) => Some(Focus::Input),
                            Some(Focus::Input) => Some(Focus::Search),
                            Some(Focus::Search) => Some(Focus::NodeList),
                        };
                    }
                    KeyCode::BackTab => {
                        self.focus = match self.focus {
                            None => Some(Focus::NodeList),
                            Some(Focus::NodeList) => Some(Focus::Search),
                            Some(Focus::Input) => Some(Focus::Conversation),
                            Some(Focus::Conversation) => Some(Focus::NodeList),
                            Some(Focus::Search) => Some(Focus::Input),
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
                                            if let Some(selected_node) = nodes.get(selected_index) {
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
                                    KeyCode::Char('h') | KeyCode::Left => {
                                        self.horizontal_scroll_state.prev();
                                    }
                                    KeyCode::Char('l') | KeyCode::Right => {
                                        self.horizontal_scroll_state.next()
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
                                Focus::Search => match key.code {
                                    KeyCode::Char(c) => {
                                        self.search.push(c);
                                    }
                                    KeyCode::Backspace => {
                                        self.search.pop();
                                    }
                                    KeyCode::Enter => {
                                        self.search.push('\n');
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
            if last_tick.elapsed() >= tick_rate {
                last_tick = Instant::now();
            }
        }
    }

    fn create_layout(area: Rect) -> (Rect, Rect, Rect, Rect, Rect) {
        let horizontal_chunks =
            Layout::horizontal([Constraint::Percentage(30), Constraint::Percentage(70)])
                .split(area);

        let right_side = Layout::vertical([
            Constraint::Min(1),
            Constraint::Percentage(10),
            Constraint::Percentage(90),
        ])
        .split(horizontal_chunks[1]);

        let left_side = Layout::vertical([Constraint::Percentage(10), Constraint::Percentage(90)])
            .split(horizontal_chunks[0]);

        (
            left_side[0],
            left_side[1],
            right_side[0],
            right_side[1],
            right_side[2],
        )
    }

    fn draw_nodes(&mut self, node_rect: Rect, frame: &mut Frame) {
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
                // Render specially if the current node is selected.
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
        frame.render_stateful_widget(list, node_rect, &mut self.node_list_state);
    }

    fn draw_conversation(&mut self, conv_rect: Rect, frame: &mut Frame) {
        let title = if let Some(contact) = &self.current_contact {
            format!("CONNECTED: {}", contact.user.as_ref().unwrap().long_name)
        } else {
            "NO NODE CONNECTED".to_string()
        };

        let conversation = Paragraph::new(vec![].clone()).gray().block(
            Block::bordered()
                .gray()
                .title(title.as_str().bold())
                .border_style(if self.focus == Some(Focus::Conversation) {
                    Style::default().fg(Color::Yellow)
                } else {
                    Style::default()
                }),
        );
        frame.render_widget(conversation, conv_rect);
        frame.render_stateful_widget(
            Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("#"))
                .end_symbol(Some("#")),
            conv_rect,
            &mut self.vertical_scroll_state,
        );
    }

    fn draw(&mut self, frame: &mut Frame) {
        // Screen frame layout.
        let area = frame.area();
        let (search_rect, node_rect, title_rect, input_rect, conv_rect) = Self::create_layout(area);

        self.draw_nodes(node_rect, frame);
        self.draw_conversation(conv_rect, frame);

        let title = Block::new()
            .title_alignment(Alignment::Center)
            .title("MESHCOM 0.0.1".bold());
        frame.render_widget(title, title_rect);

        let search_box = Paragraph::new(self.search.as_str())
            .block(Block::bordered().title("SEARCH".bold()).border_style(
                if self.focus == Some(Focus::Search) {
                    Style::default().fg(Color::Yellow)
                } else {
                    Style::default()
                },
            ))
            .wrap(Wrap { trim: false });

        frame.render_widget(search_box, search_rect);

        let input_box = Paragraph::new(self.input.as_str())
            .block(Block::bordered().title("INPUT".bold()).border_style(
                if self.focus == Some(Focus::Input) {
                    Style::default().fg(Color::Yellow)
                } else {
                    Style::default()
                },
            ))
            .wrap(Wrap { trim: false });
        frame.render_widget(input_box, input_rect);

        if self.focus == Some(Focus::Input) {
            let input_width = input_rect.width.saturating_sub(2); // Subtract 2 for borders
            let line_count = (self.input.len() as u16 / input_width) + 1;
            let cursor_x = input_rect.x + (self.input.len() as u16 % input_width) + 1;
            let cursor_y = input_rect.y + line_count;
            frame.set_cursor_position((cursor_x, cursor_y));
        }
    }
}
