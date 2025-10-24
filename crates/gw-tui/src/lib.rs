use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Frame, Terminal,
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
};
use std::io;

pub struct TuiApp {
    selected_tab: usize,
    status: gw_core::NetworkStatus,
}

impl TuiApp {
    pub fn new() -> Self {
        Self {
            selected_tab: 0,
            status: gw_core::NetworkStatus::new(),
        }
    }

    pub async fn run(&mut self) -> Result<()> {
        // Setup terminal
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        // Run the app
        let res = self.run_loop(&mut terminal).await;

        // Restore terminal
        disable_raw_mode()?;
        execute!(
            terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )?;
        terminal.show_cursor()?;

        if let Err(err) = res {
            println!("Error: {:?}", err);
        }

        Ok(())
    }

    async fn run_loop<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> Result<()> {
        loop {
            // Refresh status
            self.refresh_status().await?;

            // Draw UI
            terminal.draw(|f| self.ui(f))?;

            // Handle input
            if event::poll(std::time::Duration::from_millis(100))? {
                if let Event::Key(key) = event::read()? {
                    match key.code {
                        KeyCode::Char('q') => return Ok(()),
                        KeyCode::Char('r') => {
                            // Refresh
                        }
                        KeyCode::Tab => {
                            self.selected_tab = (self.selected_tab + 1) % 3;
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    async fn refresh_status(&mut self) -> Result<()> {
        use gw_dhcpdns::LeaseReader;
        use gw_nft::NftStatusCollector;
        use gw_nl::StatusCollector;

        let bridge_collector = StatusCollector::new().await?;
        self.status.bridges = bridge_collector.collect_bridge_status().await?;

        let nft_collector = NftStatusCollector::new();
        self.status.nftables = nft_collector.collect_table_status().await?;

        let lease_reader = LeaseReader::new();
        self.status.dhcp_leases = lease_reader.read_default_leases()?;

        Ok(())
    }

    fn ui(&self, f: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(0),
                Constraint::Length(3),
            ])
            .split(f.area());

        // Header
        let title = Paragraph::new("Ghostwarden TUI")
            .style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(title, chunks[0]);

        // Main content
        match self.selected_tab {
            0 => self.render_bridges(f, chunks[1]),
            1 => self.render_nftables(f, chunks[1]),
            2 => self.render_leases(f, chunks[1]),
            _ => {}
        }

        // Footer
        let footer_text = Line::from(vec![
            Span::raw("Tab: Switch | "),
            Span::raw("r: Refresh | "),
            Span::styled("q: Quit", Style::default().fg(Color::Red)),
        ]);
        let footer =
            Paragraph::new(footer_text).block(Block::default().borders(Borders::ALL).title("Help"));
        f.render_widget(footer, chunks[2]);
    }

    fn render_bridges(&self, f: &mut Frame, area: ratatui::layout::Rect) {
        let items: Vec<ListItem> = self
            .status
            .bridges
            .iter()
            .map(|b| {
                let content = format!("{} [{}] - {}", b.name, b.state, b.addresses.join(", "));
                ListItem::new(content)
            })
            .collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .title(format!("Bridges ({}) [Tab 1/3]", self.status.bridges.len()))
                    .borders(Borders::ALL)
                    .style(if self.selected_tab == 0 {
                        Style::default().fg(Color::Yellow)
                    } else {
                        Style::default()
                    }),
            )
            .highlight_style(Style::default().add_modifier(Modifier::BOLD));

        f.render_widget(list, area);
    }

    fn render_nftables(&self, f: &mut Frame, area: ratatui::layout::Rect) {
        let items: Vec<ListItem> = self
            .status
            .nftables
            .iter()
            .map(|t| {
                let content = format!(
                    "{} ({}) - {} chains, {} rules",
                    t.name, t.family, t.chains, t.rules
                );
                ListItem::new(content)
            })
            .collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .title(format!(
                        "nftables ({}) [Tab 2/3]",
                        self.status.nftables.len()
                    ))
                    .borders(Borders::ALL)
                    .style(if self.selected_tab == 1 {
                        Style::default().fg(Color::Yellow)
                    } else {
                        Style::default()
                    }),
            )
            .highlight_style(Style::default().add_modifier(Modifier::BOLD));

        f.render_widget(list, area);
    }

    fn render_leases(&self, f: &mut Frame, area: ratatui::layout::Rect) {
        let items: Vec<ListItem> = self
            .status
            .dhcp_leases
            .iter()
            .map(|l| {
                let hostname = l
                    .hostname
                    .as_ref()
                    .map(|h| format!(" ({})", h))
                    .unwrap_or_default();
                let content = format!("{}{} - {}", l.ip, hostname, l.mac);
                ListItem::new(content)
            })
            .collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .title(format!(
                        "DHCP Leases ({}) [Tab 3/3]",
                        self.status.dhcp_leases.len()
                    ))
                    .borders(Borders::ALL)
                    .style(if self.selected_tab == 2 {
                        Style::default().fg(Color::Yellow)
                    } else {
                        Style::default()
                    }),
            )
            .highlight_style(Style::default().add_modifier(Modifier::BOLD));

        f.render_widget(list, area);
    }
}

impl Default for TuiApp {
    fn default() -> Self {
        Self::new()
    }
}
