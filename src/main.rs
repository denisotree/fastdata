// src/main.rs

mod data_loader;
mod virtual_table;
mod tui_app;

use data_loader::{get_loader};
use virtual_table::VirtualTable;
use tui_app::TuiApp;

use std::env;
use std::error::Error;
use std::io::{self};
use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();

    let mut file_path = String::new();
    let mut backend_ext = None;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "-b" => {
                if i + 1 < args.len() {
                    backend_ext = Some(args[i + 1].clone());
                    i += 1;
                } else {
                    eprintln!("Error: '-b' option requires format specification");
                    return Ok(());
                }
            }
            _ => {
                if file_path.is_empty() {
                    file_path = args[i].clone();
                } else {
                    eprintln!("Error: Multiple files specified. Only one file expected.");
                    return Ok(());
                }
            }
        }
        i += 1;
    }

    if file_path.is_empty() {
        eprintln!("Usage: fastdata [-b format] <path_to_file>");
        return Ok(());
    }


    let extension = if let Some(ext) = backend_ext {
        ext
    } else {
        std::path::Path::new(&file_path)
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("")
            .to_string()
    };


    let loader = match get_loader(&extension) {
        Ok(loader) => loader,
        Err(e) => {
            eprintln!("Error: {}", e);
            return Ok(());
        }
    };


    let data = loader.load(&file_path)?;
    let table = VirtualTable::new(data);
    let app = TuiApp::new(table);


    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;


    terminal.hide_cursor()?;


    let mut app_stack = vec![app];


    while let Some(current_app) = app_stack.last_mut() {
        if let Some(new_app) = current_app.main_loop(&mut terminal)? {
            app_stack.push(new_app);
        } else {
            app_stack.pop();
        }
    }


    terminal.show_cursor()?;


    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
    )?;
    Ok(())
}