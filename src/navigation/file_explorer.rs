use std::{
    env::home_dir,
    fs,
    io::{self, Write, stdout},
    path::{Path, PathBuf},
};

use crossterm::{
    cursor::MoveTo,
    event::{Event, KeyCode, KeyEventKind},
    execute, queue,
    style::{Attribute, Color, Print, ResetColor, SetAttribute, SetForegroundColor},
    terminal::{self, Clear, ClearType, disable_raw_mode, enable_raw_mode},
};

struct RawModeGuard;

impl Drop for RawModeGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
    }
}

struct ExplorerViewport {
    rows: u16,
    columns: u16,
    selected: usize,
    scroll_offset: usize,
}

impl ExplorerViewport {
    fn new(columns: u16, rows: u16) -> Self {
        Self {
            rows,
            columns,
            selected: 0,
            scroll_offset: 0,
        }
    }

    fn visible_entry_count(&self) -> usize {
        // One row for the current path and one for the key hints.
        self.rows.saturating_sub(2) as usize
    }

    fn keep_selection_visible(&mut self) {
        let visible = self.visible_entry_count();
        if visible == 0 || self.selected < self.scroll_offset {
            self.scroll_offset = self.selected;
        } else if self.selected >= self.scroll_offset.saturating_add(visible) {
            self.scroll_offset = self.selected.saturating_add(1).saturating_sub(visible);
        }
    }

    fn reset_selection(&mut self) {
        self.selected = 0;
        self.scroll_offset = 0;
    }
}

fn read_directory(directory_path: &Path) -> io::Result<Vec<PathBuf>> {
    let mut entry_paths = fs::read_dir(directory_path)?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.is_file() || path.is_dir())
        .collect::<Vec<_>>();

    // A stable order makes the cursor predictable between redraws. Directories
    // are grouped first, like most graphical file explorers.
    entry_paths.sort_by(|left, right| right.is_dir().cmp(&left.is_dir()).then_with(|| left.file_name().cmp(&right.file_name())));

    Ok(entry_paths)
}

fn explorer_entries(directory_path: &Path, allow_navigation: bool) -> io::Result<Vec<PathBuf>> {
    let mut entry_paths = read_directory(directory_path)?;
    if !allow_navigation {
        entry_paths.retain(|path| path.is_file());
    }
    Ok(entry_paths)
}

fn clipped(text: &str, width: usize) -> String {
    text.chars().take(width).collect()
}

#[allow(clippy::cast_possible_truncation)]
fn display_directory(output: &mut impl Write, directory_path: &Path, entry_paths: &[PathBuf], viewport: &ExplorerViewport, allow_navigation: bool) -> io::Result<()> {
    queue!(output, MoveTo(0, 0), Clear(ClearType::All))?;

    let width = viewport.columns as usize;
    queue!(output, SetForegroundColor(Color::Yellow), Print(clipped(&directory_path.display().to_string(), width)), ResetColor)?;

    let visible = viewport.visible_entry_count();
    for (screen_row, (entry_index, entry_path)) in entry_paths.iter().enumerate().skip(viewport.scroll_offset).take(visible).enumerate() {
        let is_selected = entry_index == viewport.selected;
        let is_directory = entry_path.is_dir();
        let mut name = entry_path.file_name().map_or_else(|| entry_path.display().to_string(), |name| name.to_string_lossy().into_owned());
        if is_directory {
            name.push('/');
        }

        let prefix = if is_selected { "> " } else { "  " };
        let line = clipped(&format!("{prefix}{name}"), width);
        queue!(output, MoveTo(0, (screen_row as u16).saturating_add(1)))?;
        if is_selected {
            queue!(output, SetAttribute(Attribute::Reverse))?;
        }
        queue!(output, SetForegroundColor(if is_directory { Color::Cyan } else { Color::White }), Print(line), ResetColor)?;
        if is_selected {
            queue!(output, SetAttribute(Attribute::Reset))?;
        }
    }

    if viewport.rows > 1 {
        let hint = if entry_paths.is_empty() && allow_navigation {
            "Empty directory  ←: parent  q: quit"
        } else if entry_paths.is_empty() {
            "No stored files  q: quit"
        } else if allow_navigation {
            "↑/↓: select  →: open  ←: parent  Enter: choose  q: quit"
        } else {
            "↑/↓: select  Enter: choose  q: quit"
        };
        queue!(
            output,
            MoveTo(0, viewport.rows.saturating_sub(1)),
            SetForegroundColor(Color::DarkGrey),
            Print(clipped(hint, width)),
            ResetColor
        )?;
    }

    output.flush()
}

fn enable_file_explorer(mut current_directory: PathBuf, allow_navigation: bool) -> io::Result<Option<PathBuf>> {
    let mut entry_paths = explorer_entries(&current_directory, allow_navigation)?;
    let (columns, rows) = terminal::size()?;
    let mut viewport = ExplorerViewport::new(columns, rows);

    enable_raw_mode()?;
    let raw_mode = RawModeGuard;
    let mut output = stdout();
    display_directory(&mut output, &current_directory, &entry_paths, &viewport, allow_navigation)?;

    loop {
        let event = crossterm::event::read()?;
        match event {
            Event::Resize(columns, rows) => {
                viewport.columns = columns;
                viewport.rows = rows;
                viewport.keep_selection_visible();
            },
            Event::Key(key_event) if key_event.kind != KeyEventKind::Release => match key_event.code {
                KeyCode::Up if viewport.selected > 0 => {
                    viewport.selected = viewport.selected.saturating_sub(1);
                    viewport.keep_selection_visible();
                },
                KeyCode::Down if viewport.selected.saturating_add(1) < entry_paths.len() => {
                    viewport.selected = viewport.selected.saturating_add(1);
                    viewport.keep_selection_visible();
                },
                KeyCode::Right if allow_navigation => {
                    if let Some(path) = entry_paths.get(viewport.selected).filter(|path| path.is_dir()) {
                        current_directory.clone_from(path);
                        entry_paths = read_directory(&current_directory)?;
                        viewport.reset_selection();
                    }
                },
                KeyCode::Left if allow_navigation => {
                    if let Some(parent) = current_directory.parent() {
                        let previous_directory = current_directory.clone();
                        current_directory = parent.to_path_buf();
                        entry_paths = read_directory(&current_directory)?;
                        viewport.reset_selection();
                        if let Some(index) = entry_paths.iter().position(|path| path == &previous_directory) {
                            viewport.selected = index;
                            viewport.keep_selection_visible();
                        }
                    }
                },
                KeyCode::Enter => {
                    if let Some(path) = entry_paths.get(viewport.selected).filter(|path| path.is_file()) {
                        execute!(output, Clear(ClearType::All), MoveTo(0, 0))?;
                        drop(raw_mode);
                        return Ok(Some(path.clone()));
                    }
                },
                KeyCode::Char('q') | KeyCode::Esc => {
                    disable_raw_mode()?;
                    execute!(output, Clear(ClearType::All), MoveTo(0, 0))?;
                    return Ok(None);
                },
                _ => {},
            },
            _ => {},
        }

        display_directory(&mut output, &current_directory, &entry_paths, &viewport, allow_navigation)?;
    }
}

pub fn select_source_file() -> io::Result<Option<PathBuf>> {
    let home = home_dir().ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "home directory is unavailable"))?;
    enable_file_explorer(home, true)
}

pub fn select_vault_entry(vault_directory: &Path) -> io::Result<Option<PathBuf>> {
    enable_file_explorer(vault_directory.to_path_buf(), false)?
        .map(|path| {
            path.file_name()
                .map(PathBuf::from)
                .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "vault entry has no file name"))
        })
        .transpose()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scrolling_keeps_the_selection_in_view() {
        let mut viewport = ExplorerViewport::new(80, 5);
        viewport.selected = 4;
        viewport.keep_selection_visible();
        assert_eq!(viewport.scroll_offset, 2);

        viewport.selected = 1;
        viewport.keep_selection_visible();
        assert_eq!(viewport.scroll_offset, 1);
    }

    #[test]
    fn clipping_does_not_split_unicode_characters() {
        assert_eq!(clipped("↑/↓: select", 4), "↑/↓:");
    }

    #[test]
    #[allow(clippy::panic_in_result_fn)]
    fn flat_explorer_hides_directories() -> io::Result<()> {
        let directory = std::env::temp_dir().join(format!("secure-safe-explorer-{}", std::process::id()));
        let nested = directory.join("nested");
        let file = directory.join("entry.safe");
        fs::create_dir_all(&nested)?;
        fs::write(&file, [])?;

        let entries = explorer_entries(&directory, false)?;

        assert_eq!(entries, vec![file]);
        fs::remove_dir_all(directory)?;
        Ok(())
    }
}
