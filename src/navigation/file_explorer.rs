use std::{env::home_dir, fs, io::stdout, path::PathBuf};

use crossterm::{
    event::{Event, KeyCode},
    execute, terminal,
    terminal::{Clear, ClearType, enable_raw_mode},
};
use owo_colors::OwoColorize;

use crate::io;

#[derive(Default)]
struct SelectionCursor {
    selected_row: u8,
    selected_column: u8,
}

struct ExplorerViewport {
    terminal_rows: u8,
    terminal_columns: u8,
    selection_cursor: SelectionCursor,
}

fn read_directory(directory_path: PathBuf) -> io::Result<Vec<PathBuf>> {
    let mut entry_paths = Vec::with_capacity(3);

    for directory_entry in fs::read_dir(directory_path).unwrap() {
        let directory_entry = directory_entry.unwrap();
        let entry_path = directory_entry.path();

        entry_paths.push(entry_path);
    }

    Ok(entry_paths)
}

fn display_directory(directory_path: PathBuf) -> io::Result<Vec<PathBuf>> {
    let entry_paths = read_directory(directory_path)?;

    entry_paths.iter().for_each(|entry_path| {
        if entry_path.is_dir() {
            println!("{}", entry_path.file_name().unwrap().display().green());
        }
        if entry_path.is_file() {
            println!("{}", entry_path.file_name().unwrap().display().black())
        }
    });

    Ok(entry_paths)
}

fn current_entry_path<'a>(viewport: &ExplorerViewport, entry_paths: &'a [PathBuf]) -> &'a PathBuf {
    entry_paths.get(viewport.selection_cursor.selected_row as usize).unwrap()
}
fn open_current_directory(viewport: &ExplorerViewport, entry_paths: &[PathBuf]) -> io::Result<Vec<PathBuf>> {
    let current_path = current_entry_path(viewport, entry_paths);

    let child_entry_paths = display_directory(current_path.clone())?;

    Ok(child_entry_paths)
}
fn open_parent_directory(viewport: &ExplorerViewport, entry_paths: &[PathBuf]) -> Option<Vec<PathBuf>> {
    let current_path = current_entry_path(viewport, entry_paths);

    let parent_entry_paths = display_directory(PathBuf::from(current_path.parent()?));

    return Some(parent_entry_paths.unwrap());
}
pub fn enable_file_explorer() -> io::Result<PathBuf> {
    execute!(stdout(), Clear(ClearType::All))?;

    let (columns, rows) = terminal::size()?;
    let mut viewport = ExplorerViewport {
        terminal_columns: columns as u8,
        terminal_rows: rows as u8,
        selection_cursor: SelectionCursor::default(),
    };

    enable_raw_mode()?;

    let mut entry_paths = display_directory(home_dir().unwrap())?;
    loop {
        if let Event::Key(key_event) = crossterm::event::read()? {
            match key_event.code {
                KeyCode::Up => {
                    if viewport.selection_cursor.selected_row > 1 {
                        viewport.selection_cursor.selected_column -= 1;
                    }
                },
                KeyCode::Down => {
                    if viewport.selection_cursor.selected_column < viewport.terminal_rows {
                        viewport.selection_cursor.selected_column += 1;
                    }
                },
                KeyCode::Right if current_entry_path(&viewport, &entry_paths).is_dir() => {
                    let res = open_current_directory(&viewport, &entry_paths)?;
                    entry_paths = res;
                },

                KeyCode::Left if current_entry_path(&viewport, &entry_paths).is_dir() => {
                    let Some(res) = open_parent_directory(&viewport, &entry_paths) else {
                        continue;
                    };
                    entry_paths = res;
                },
                KeyCode::Enter if current_entry_path(&viewport, &entry_paths).is_file() => {
                    return Ok(current_entry_path(&viewport, &entry_paths).clone());
                },
                _ => (),
            }
        }
    }
}
