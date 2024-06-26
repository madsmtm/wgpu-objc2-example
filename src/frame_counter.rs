use std::{
    sync::{Arc, Mutex},
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

#[derive(Debug)]
pub struct FrameCounter {
    state: Arc<Mutex<FrameCounterState>>,
    // Helper thread to ensure that we print the FPS every second, even if it's 0.
    printer_thread: Option<JoinHandle<()>>,
}

impl Drop for FrameCounter {
    fn drop(&mut self) {
        self.printer_thread
            .take()
            .expect("printer thread set")
            .join()
            .expect("printer thread");
    }
}

#[derive(Debug)]
struct FrameCounterState {
    last_printed_instant: Instant,
    frame_count: u32,
}

impl FrameCounter {
    pub fn new() -> Self {
        let state = FrameCounterState {
            last_printed_instant: Instant::now(),
            frame_count: 0,
        };
        let state: Arc<Mutex<FrameCounterState>> = Arc::new(Mutex::new(state));
        let state_clone = Arc::clone(&state);
        Self {
            state,
            printer_thread: Some(thread::spawn(move || loop {
                state_clone.lock().unwrap().print();
                thread::sleep(Duration::from_millis(100));
            })),
        }
    }

    pub fn update(&self) {
        self.state.lock().unwrap().update();
    }
}

impl FrameCounterState {
    fn update(&mut self) {
        self.frame_count += 1;
        self.print();
    }

    fn print(&mut self) {
        let now = Instant::now();
        let elapsed = now - self.last_printed_instant;
        if elapsed > Duration::from_secs(1) {
            let fps = self.frame_count as f32 / elapsed.as_secs_f32();
            tracing::info!("FPS: {:.1}", fps);

            self.last_printed_instant = now;
            self.frame_count = 0;
        }
    }
}
