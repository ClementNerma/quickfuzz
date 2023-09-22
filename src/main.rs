#![forbid(unsafe_code)]
#![forbid(unused_must_use)]
#![warn(unused_crate_dependencies)]

use std::{error::Error, io, process::ExitCode};

use crossterm::{
    event::{self, Event, KeyCode},
    terminal::{self, disable_raw_mode},
    ExecutableCommand,
};
use ratatui::{
    prelude::{Backend, Constraint, CrosstermBackend, Direction, Layout},
    style::{Color, Style},
    widgets::{List, ListItem, ListState, Paragraph},
    Frame, Terminal,
};
use tui_input::{backend::crossterm::EventHandler, Input};

fn main() -> ExitCode {
    match inner_main() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("{err}");
            ExitCode::FAILURE
        }
    }
}

fn inner_main() -> Result<(), Box<dyn Error>> {
    let list = io::stdin().lines().collect::<Result<Vec<_>, _>>()?;

    crossterm::terminal::enable_raw_mode()?;

    let mut stdout = io::stdout();

    stdout
        .execute(terminal::EnterAlternateScreen)?
        .execute(event::EnableMouseCapture)?;

    let backend = CrosstermBackend::new(stdout);

    let mut terminal = Terminal::new(backend)?;

    let chosen = run_app(
        &mut terminal,
        State {
            input_widget: Input::default(),
            list,
            list_state: ListState::default(),
            filtered: vec![],
        },
    )?;

    disable_raw_mode()?;

    terminal
        .backend_mut()
        .execute(terminal::LeaveAlternateScreen)?
        .execute(event::DisableMouseCapture)?;

    terminal.show_cursor()?;

    print!("{chosen}");

    Ok(())
}

fn run_app<B: Backend>(
    terminal: &mut Terminal<B>,
    mut state: State,
) -> Result<String, Box<dyn Error>> {
    loop {
        state.filtered = fuzzy_find(state.input_widget.value(), &state.list);

        match state.list_state.selected() {
            Some(selected) => {
                if selected >= state.filtered.len() {
                    state
                        .list_state
                        .select(Some(state.filtered.len().max(1) - 1));
                }
            }

            None => {
                if !state.filtered.is_empty() {
                    state.list_state.select(Some(0));
                }
            }
        }

        terminal.draw(|f| draw_ui(f, &mut state))?;

        match event::read()? {
            Event::Key(key) => match key.code {
                KeyCode::Enter => {
                    if let Some(selected) = state.list_state.selected() {
                        return Ok(state.filtered[selected].clone());
                    }
                }

                KeyCode::Esc => {
                    return Err("User cancelled".into());
                }

                KeyCode::Up => match state.list_state.selected() {
                    Some(selected) => {
                        if selected > 0 {
                            state.list_state.select(Some(selected - 1));
                        }
                    }

                    None => {
                        if !state.filtered.is_empty() {
                            state.list_state.select(Some(state.filtered.len() - 1));
                        }
                    }
                },

                KeyCode::Down => match state.list_state.selected() {
                    Some(selected) => {
                        if selected + 1 < state.filtered.len() {
                            state.list_state.select(Some(selected + 1));
                        }
                    }

                    None => {
                        if !state.filtered.is_empty() {
                            state.list_state.select(Some(0));
                        }
                    }
                },

                _ => {
                    state.input_widget.handle_event(&Event::Key(key));
                }
            },

            Event::Mouse(_) => todo!(),

            _ => {}
        }
    }
}

fn draw_ui<B: Backend>(f: &mut Frame<B>, state: &mut State) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(10)])
        .split(f.size());

    // === Draw input line === //

    let scroll = state.input_widget.visual_scroll(
        (
            // Keep 1 space for cursor
            chunks[0].width.max(1) - 1
        ) as usize,
    );

    let input = Paragraph::new(state.input_widget.value()).scroll((0, scroll as u16));

    f.render_widget(input, chunks[0]);

    f.set_cursor(
        chunks[0].x + (state.input_widget.visual_cursor().max(scroll) - scroll) as u16,
        chunks[0].y,
    );

    // === Draw results list === //

    let results = state
        .filtered
        .iter()
        .cloned()
        .map(ListItem::new)
        .collect::<Vec<_>>();

    let results = List::new(results).highlight_style(Style::default().bg(Color::Black));

    f.render_stateful_widget(results, chunks[1], &mut state.list_state);
}

fn fuzzy_find(query: &str, list: &[String]) -> Vec<String> {
    if query.is_empty() {
        return list.to_vec();
    }

    let mut scores = list
        .iter()
        .enumerate()
        .map(|(i, result)| (i, compute_fuzzy_find_score(query, result)))
        .filter(|(_, score)| *score > 0)
        .collect::<Vec<_>>();

    scores.sort_by_key(|(_, score)| *score);

    scores
        .into_iter()
        .map(|(i, _)| list.get(i).unwrap())
        .cloned()
        .collect()
}

fn compute_fuzzy_find_score(query: &str, subject: &str) -> usize {
    query
        .chars()
        .map(|c| subject.chars().filter(|cc| c == *cc).count())
        .sum()
}

struct State {
    input_widget: Input,
    list: Vec<String>,
    list_state: ListState,
    filtered: Vec<String>,
}
