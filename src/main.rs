extern crate alpm;
extern crate ratatui;

use core::fmt;
use std::io::{self, stdout};

use alpm::{Alpm, PackageReason};
use ratatui::{
    backend::CrosstermBackend, crossterm::{
        event::{self, KeyCode, KeyEventKind},
        terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
        ExecutableCommand,
    }, style::{Color, Modifier, Style}, widgets::{Block, List, ListDirection, ListState}, Terminal
};

enum DedepAction {
    Explicit,
    Nothing,
    Remove,
}

impl DedepAction {
    fn as_char(&self) -> char {
        match self {
            DedepAction::Explicit => 'E',
            DedepAction::Nothing => ' ',
            DedepAction::Remove => 'R',
        }
    }
}

struct DedepPackage<'a> {
    package: &'a alpm::Package,
    action: DedepAction,
}

impl<'a> DedepPackage<'a> {
    fn stage_action(&mut self, action: DedepAction) {
        self.action = action;
    }
}

impl<'a> fmt::Display for DedepPackage<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "[{}] {}", self.action.as_char(), self.package.name())
    }
}

fn main() -> Result<(), io::Error> {
    let alpm = Alpm::new("/", "/var/lib/pacman").expect("Failed to initialize ALPM");
    let local_db = alpm.localdb();

    let all_orphan_pkgs = local_db
        .pkgs()
        .iter()
        .filter(|p| p.reason() == PackageReason::Depend && p.required_by().len() == 0);

    let mut all_orphan_dedep_pkgs: Vec<DedepPackage> = all_orphan_pkgs
        .map(|p| DedepPackage {
            package: p,
            action: DedepAction::Nothing,
        })
        .collect::<Vec<_>>();
    let (mut orphan_ddpkgs, mut _pseudo_orphan_ddpkgs): (Vec<&mut DedepPackage>, Vec<&mut DedepPackage>) = all_orphan_dedep_pkgs
        .iter_mut()
        .partition(|ddp| ddp.package.optional_for().len() == 0);

    stdout().execute(EnterAlternateScreen)?;
    enable_raw_mode()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;
    terminal.clear()?;

    let mut state = ListState::default();

    loop {
        terminal.draw(|frame| {
            let list = List::new(orphan_ddpkgs.iter().map(|p| format!("{}", p)))
                .block(Block::bordered().title("List"))
                .style(Style::default().fg(Color::White))
                .highlight_style(Style::default().add_modifier(Modifier::ITALIC))
                .highlight_symbol(">>")
                .repeat_highlight_symbol(true)
                .direction(ListDirection::TopToBottom);

            let area = frame.size();
            frame.render_stateful_widget(list, area, &mut state);

        })?;
        if event::poll(std::time::Duration::from_millis(16))? {
            if let event::Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        // global actions
                        KeyCode::Char('q') => break,
                        // movement
                        KeyCode::Char('k') => state.select_previous(),
                        KeyCode::Char('j') => state.select_next(),
                        // stage action
                        KeyCode::Char('E') => {
                            if let Some(selected_index) = state.selected() {
                                orphan_ddpkgs.get_mut(selected_index).unwrap().stage_action(DedepAction::Explicit);
                            }
                        }
                        _ => {},
                    };
                }
                if key.kind == KeyEventKind::Press && key.code == KeyCode::Char('q') {
                    break;
                }
            }
        }
    }

    stdout().execute(LeaveAlternateScreen)?;
    disable_raw_mode()?;
    Ok(())
}
