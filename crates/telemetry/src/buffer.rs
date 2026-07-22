//! Telemetry buffer — in-memory ring buffer for events.

use layermind_shared::event::Envelope;

pub struct RingBuffer {
    buf: Vec<Envelope>,
    capacity: usize,
}

impl RingBuffer {
    pub fn new(capacity: usize) -> Self {
        Self {
            buf: Vec::with_capacity(capacity),
            capacity,
        }
    }

    pub fn push(&mut self, event: Envelope) {
        if self.buf.len() >= self.capacity {
            self.buf.remove(0);
        }
        self.buf.push(event);
    }

    pub fn drain(&mut self) -> Vec<Envelope> {
        std::mem::replace(&mut self.buf, Vec::with_capacity(self.capacity))
    }

    pub fn len(&self) -> usize {
        self.buf.len()
    }
}
