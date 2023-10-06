use core::time;
use std::{
    sync::{Arc, Mutex},
    thread,
};

use anyhow::Context;
use ratatui::{
    prelude::CrosstermBackend,
    style::Style,
    symbols,
    Terminal,
    text::Span,
    widgets::{Axis, Block, Borders, Dataset, GraphType},
    widgets::Chart,
};
use ratatui::layout::Rect;
use ratatui::widgets::{Paragraph, Wrap};

use crate::{
    model::CpuFrequencyHistory,
    sysfs,
};

/// This is a frequency monitor chart UI experiment
pub(crate) struct Monitor {}

const POLL_RATE: u64 = 500;
const FREQ_SCALER: f64 = 1000000.0;
const MAX_ANSI_COLORS: u32 = 8; // skip 0 = black

fn style_for_cpu(i: u32) -> Style {
    Style::default().fg(ratatui::style::Color::Indexed(
        (i % MAX_ANSI_COLORS) as u8 + 1, // skip 0 = black
    ))
}

impl Monitor {
    pub(crate) fn start() -> anyhow::Result<()> {
        let cpus = sysfs::get_cpus()?;
        let cpu = cpus.first().context("Should have one CPU")?;
        let min_cpu = sysfs::read_int_value(&cpu.path_for(sysfs::CPU_MIN_FREQ))?;
        let max_cpu = sysfs::read_int_value(&cpu.path_for(sysfs::CPU_MAX_FREQ))?;

        let freq_data_handle = Arc::new(Mutex::new(CpuFrequencyHistory::new(
            cpus.len(),
            30,
            min_cpu,
            max_cpu,
        )));

        let freq_data_for_polling = freq_data_handle.clone();
        let freq_polling_thread = thread::spawn(move || -> anyhow::Result<()> {
            loop {
                let mut new_data = vec![0u32; cpus.len()];

                for cpu in cpus.iter() {
                    let new_value = sysfs::read_int_value(&cpu.path_for(sysfs::CPU_CUR_FREQ))
                        .context("Failed to read current frequency")?;
                    new_data[cpu.0 as usize] = new_value;
                }

                {
                    // Lock scope
                    let freq_data_guard = freq_data_for_polling.lock();
                    let mut freq_data = freq_data_guard.expect("Lock failed");
                    freq_data.append(new_data);
                    if !freq_data.running {
                        break;
                    }
                } // lock scope
                thread::sleep(time::Duration::from_millis(POLL_RATE));
            }
            Ok(())
        });

        crossterm::terminal::enable_raw_mode()?;
        crossterm::execute!(std::io::stderr(), crossterm::terminal::EnterAlternateScreen)?;

        let mut terminal = Terminal::new(CrosstermBackend::new(std::io::stderr()))?;

        let freq_data_for_loop = freq_data_handle.clone();
        let mut hist: CpuFrequencyHistory;
        loop {
            {
                // Lock scope
                let freq_data_guard = freq_data_for_loop.lock();
                hist = freq_data_guard.expect("Lock failed").clone();
            } // Lock scope

            let numbers: Vec<Vec<(f64, f64)>> = hist
                .data
                .iter()
                .map(|freqs| {
                    freqs
                        .iter()
                        .enumerate()
                        .map(|(j, freq)| (f64::from(j as u32), f64::from(*freq) / FREQ_SCALER))
                        .collect()
                })
                .collect();

            let min_value = f64::from(hist.min_value) / FREQ_SCALER;
            let max_value = f64::from(hist.max_value) / FREQ_SCALER;

            let sub_range: f64 = (max_value - min_value) / 5.0;
            let range: Vec<_> = (1..=5)
                .map(|i| f64::from(i) * sub_range)
                .map(|v| Span::from(format!("{:.1}", v)))
                .collect();

            let curr_freq: Vec<_> = numbers
                .iter()
                .map(|freq_hist| {
                    let (_, last_value) = *freq_hist.last().unwrap();
                    last_value
                })
                .collect();

            let avg_freq = curr_freq.iter().sum::<f64>() / f64::from(curr_freq.len() as u16);

            let datasets = (0..numbers.len())
                .map(|i| {
                    Dataset::default()
                        .name(format!("cpu{} - {:.1} GHz", i, curr_freq[i]))
                        .marker(symbols::Marker::Braille)
                        .graph_type(GraphType::Line)
                        .style(style_for_cpu(i as u32))
                        .data(&numbers[i])
                })
                .collect();

            terminal.draw(|f| {
                let chart = Chart::new(datasets)
                    .block(
                        Block::default()
                            .title("CPU Frequency (q or ESC to quit)")
                            .borders(Borders::ALL),
                    )
                    .x_axis(Axis::default().bounds([0.0, f64::from(hist.history as u32)]))
                    .y_axis(
                        Axis::default()
                            .title("GHz")
                            .bounds([min_value, max_value])
                            .labels(range),
                    );

                f.render_widget(chart, f.size());


                // overlay a block with a line for each cpu with the current frequency with the matching style

                f.render_widget(
                    Paragraph::new(ratatui::text::Line::from(curr_freq.iter()
                        .enumerate()
                        .map(|(i, freq)| {
                            Span::styled(
                                format!("CPU{:2}: {:.1} ", i, freq),
                                style_for_cpu(i as u32),
                            )
                        })
                        .collect::<Vec<Span>>())
                    ).wrap(Wrap { trim: true })
                        .block(
                            Block::default()
                                .title(format!("Current Frequency (avg: {:.1})", avg_freq))
                                .borders(Borders::ALL)),
                    Rect::new(f.size().width - 46, f.size().y, 46, (2 + numbers.len() / 4) as u16),
                );
            })?;


            if crossterm::event::poll(time::Duration::from_millis(POLL_RATE))? {
                if let crossterm::event::Event::Key(key) = crossterm::event::read()? {
                    if key.code == crossterm::event::KeyCode::Char('q')
                        || key.code == crossterm::event::KeyCode::Esc
                    {
                        {
                            let freq_data_guard = freq_data_for_loop.lock();
                            let mut freq_data = freq_data_guard.unwrap();
                            freq_data.running = false;
                        }
                        break;
                    }
                }
            }
        }

        freq_polling_thread.join().unwrap()?;

        crossterm::execute!(std::io::stderr(), crossterm::terminal::LeaveAlternateScreen)?;
        crossterm::terminal::disable_raw_mode()?;

        Ok(())
    }
}
