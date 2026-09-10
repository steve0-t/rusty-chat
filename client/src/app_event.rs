use anyhow::Result;
use ratatui::crossterm::event::{self, Event as CrosstermEvent, KeyEvent, MouseEvent};
use std::{
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

/// terminal events
#[derive(Clone, Copy, Debug)]
pub enum AppEvent {
    /// Terminal tick
    Tick,

    /// Key press
    Key(KeyEvent),

    /// Mouse click/scroll
    Mouse(MouseEvent),

    /// Terminal resize
    Resize(u16, u16),
}

/// terminal event handler.
#[derive(Debug)]
#[allow(dead_code)]
pub struct EventHandler {
    sender: mpsc::Sender<AppEvent>,
    receiver: mpsc::Receiver<AppEvent>,
    handler: thread::JoinHandle<()>,
}

impl EventHandler {
    /// constructs a new instance of [`EventHandler`].
    pub fn new(tick_rate: u64) -> Self {
        let tick_rate = Duration::from_millis(tick_rate);
        let (sender, receiver) = mpsc::channel();
        let handler = {
            let sender = sender.clone();
            thread::spawn(move || {
                let mut last_tick = Instant::now();
                loop {
                    let timeout = tick_rate
                        .checked_sub(last_tick.elapsed())
                        .unwrap_or(tick_rate);

                    if event::poll(timeout).expect("unable to poll for event") {
                        match event::read().expect("unable to read event") {
                            CrosstermEvent::Key(e) => {
                                if e.kind == event::KeyEventKind::Press {
                                    sender.send(AppEvent::Key(e))
                                } else {
                                    Ok(()) // ignore KeyEventKind::Release on windows
                                }
                            }
                            CrosstermEvent::Mouse(e) => sender.send(AppEvent::Mouse(e)),
                            CrosstermEvent::Resize(w, h) => sender.send(AppEvent::Resize(w, h)),
                            _ => unimplemented!(),
                        }
                        .expect("failed to send terminal event")
                    }

                    if last_tick.elapsed() >= tick_rate {
                        sender
                            .send(AppEvent::Tick)
                            .expect("failed to send tick event");
                        last_tick = Instant::now();
                    }
                }
            })
        };
        Self {
            sender,
            receiver,
            handler,
        }
    }

    /// receive the next event from the handler thread
    /// blocks until data is available
    pub fn next(&self) -> Result<AppEvent> {
        Ok(self.receiver.recv()?)
    }
}
