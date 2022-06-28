use std::borrow::Cow;
use std::io::{self, stdout, Stdout, Write};

use anyhow::anyhow;
use contec_protocol::incoming_package::RealTimeData;
use crossterm::event::{Event, EventStream};
use crossterm::{event, execute, terminal};
use futures::future::LocalBoxFuture;
use futures::{FutureExt, StreamExt};
use tui::backend::CrosstermBackend;
use tui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use tui::style::{Color, Modifier, Style};
use tui::text::{Span, Spans, Text};
use tui::widgets::{Axis, Block, Borders, Chart, Dataset, Paragraph};
use tui::{symbols, Frame, Terminal};

pub trait RealtimeTerminal: Sized {
    fn new() -> anyhow::Result<Self>;
    fn close(&mut self) -> anyhow::Result<()>;
    fn handle_event(&mut self) -> LocalBoxFuture<anyhow::Result<Event>>;
    fn add_message(&mut self, message: impl AsRef<str>) -> anyhow::Result<()>;
    fn clear_messages(&mut self) -> anyhow::Result<()>;
    fn next_sample(&mut self, sample: RealTimeData);
    fn update(&mut self) -> anyhow::Result<()>;
}

pub struct MinTerminal {
    events: EventStream,
    count: usize,
    last_sample: Option<RealTimeData>,
}

impl RealtimeTerminal for MinTerminal {
    fn new() -> anyhow::Result<Self> {
        Ok(Self {
            events: EventStream::new(),
            count: 0,
            last_sample: None,
        })
    }

    fn close(&mut self) -> anyhow::Result<()> {
        Ok(())
    }

    fn handle_event(&mut self) -> LocalBoxFuture<anyhow::Result<Event>> {
        async {
            match self.events.next().await {
                Some(Ok(event)) => Ok(event),
                Some(Err(err)) => Err(err.into()),
                None => Err(anyhow!("Unexpected end of stream")),
            }
        }
        .boxed_local()
    }

    fn add_message(&mut self, message: impl AsRef<str>) -> anyhow::Result<()> {
        println!("{}", message.as_ref());
        Ok(())
    }

    fn clear_messages(&mut self) -> anyhow::Result<()> {
        Ok(())
    }

    fn next_sample(&mut self, sample: RealTimeData) {
        self.last_sample = Some(sample);
        self.count += 1;
    }

    fn update(&mut self) -> anyhow::Result<()> {
        print!("Samples: {:6}\r", self.count);
        stdout().flush()?;
        Ok(())
    }
}

pub struct GraphTerminal {
    terminal: Terminal<CrosstermBackend<Stdout>>,
    events: EventStream,
    message: String,
    count: usize,
    buffer: Vec<u8>,
    last_sample: Option<RealTimeData>,
}

impl RealtimeTerminal for GraphTerminal {
    fn new() -> anyhow::Result<Self> {
        terminal::enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, terminal::EnterAlternateScreen, event::EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;

        Ok(Self {
            terminal,
            events: EventStream::new(),
            message: "".to_string(),
            count: 0,
            buffer: vec![],
            last_sample: None,
        })
    }

    fn close(&mut self) -> anyhow::Result<()> {
        terminal::disable_raw_mode()?;
        crossterm::execute!(
            self.terminal.backend_mut(),
            terminal::LeaveAlternateScreen,
            event::DisableMouseCapture
        )?;
        self.terminal.show_cursor()?;
        Ok(())
    }

    fn handle_event(&mut self) -> LocalBoxFuture<anyhow::Result<Event>> {
        async {
            match self.events.next().await {
                Some(Ok(event)) => Ok(event),
                Some(Err(err)) => Err(err.into()),
                None => Err(anyhow!("Unexpected end of stream")),
            }
        }
        .boxed_local()
    }

    fn add_message(&mut self, message: impl AsRef<str>) -> anyhow::Result<()> {
        self.message.push_str(message.as_ref());
        self.message.push('\n');
        self.update()
    }
    fn clear_messages(&mut self) -> anyhow::Result<()> {
        self.message = "".to_string();
        self.update()
    }
    fn next_sample(&mut self, sample: RealTimeData) {
        self.last_sample = Some(sample);

        if self.buffer.len() < 300 {
            self.buffer.push(sample.pulse_waveform);
        } else {
            self.buffer[self.count % 300] = sample.pulse_waveform;
        }
        self.count += 1;
    }
    fn update(&mut self) -> anyhow::Result<()> {
        let sample = if let Some(s) = self.last_sample {
            s
        } else {
            return Ok(());
        };

        let chart_data = self
            .buffer
            .iter()
            .enumerate()
            .filter(|(index, _)| !((self.count % 300)..(self.count % 300) + 10).contains(index))
            .map(|(index, value)| (f64::from(index as u16), f64::from(*value)))
            .collect::<Vec<_>>();

        self.terminal.draw(|f| {
            let [header, graph] =
                layout(f.size(), Direction::Vertical, [Constraint::Length(5), Constraint::Min(0)]);

            let [data, message] = layout(header, Direction::Horizontal, [
                Constraint::Percentage(60),
                Constraint::Percentage(40),
            ]);

            let [_, message] =
                layout(message, Direction::Horizontal, [Constraint::Length(1), Constraint::Min(0)]);

            let [header, data] =
                layout(data, Direction::Vertical, [Constraint::Length(2), Constraint::Length(3)]);

            let [pr, spo2, pi] = layout(data, Direction::Horizontal, [
                Constraint::Ratio(1, 3),
                Constraint::Ratio(1, 3),
                Constraint::Ratio(1, 3),
            ]);

            f.render_widget(
                Paragraph::new(Span::styled(
                    "Pulox",
                    Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                ))
                .alignment(Alignment::Center),
                header,
            );

            f.render_widget(
                Paragraph::new::<Vec<Spans>>(
                    self.message
                        .lines()
                        .flat_map(|l| {
                            l.chars()
                                .collect::<Vec<_>>()
                                .chunks(message.width as usize)
                                .map(|c| Span::raw(String::from_iter(c)))
                                .collect::<Vec<_>>()
                        })
                        .collect::<Vec<Span>>()
                        .into_iter()
                        .rev()
                        .take(5)
                        .rev()
                        .map(Into::into)
                        .collect(),
                ),
                message,
            );

            render_text_box(f, pr, "Pulse rate", sample.pulse_rate.to_string());
            render_text_box(f, spo2, "SpO2", sample.spo2.to_string());
            render_text_box(
                f,
                pi,
                "PI",
                if sample.pi_invalid {
                    "not supported".to_string()
                } else {
                    sample.pi.to_string()
                },
            );

            f.render_widget(
                Chart::new(vec![Dataset::default()
                    .name(format!("{} samples", self.count))
                    .marker(symbols::Marker::Braille)
                    .style(Style::default().fg(Color::Cyan))
                    .data(&chart_data)])
                .block(
                    Block::default()
                        .title(Span::styled(
                            "Pulse",
                            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                        ))
                        .borders(Borders::ALL),
                )
                .x_axis(Axis::default().bounds([0., 300.]))
                .y_axis(Axis::default().bounds([0., 128.])),
                graph,
            );
        })?;
        Ok(())
    }
}

fn render_text_box<'a>(
    f: &mut Frame<CrosstermBackend<Stdout>>,
    area: Rect,
    title: impl Into<Cow<'a, str>>,
    text: impl Into<Text<'a>>,
) {
    f.render_widget(
        Paragraph::new(text).block(
            Block::default()
                .title(Span::styled(
                    title,
                    Style::default().fg(Color::Cyan), /*.add_modifier(Modifier::BOLD)*/
                ))
                .style(Style::default().bg(Color::Black))
                .borders(Borders::all()),
        ),
        area,
    );
}

fn layout<const N: usize>(
    area: Rect,
    direction: Direction,
    constraints: [Constraint; N],
) -> [Rect; N] {
    Layout::default()
        .direction(direction)
        .constraints(constraints)
        .split(area)
        .try_into()
        .unwrap()
}
