//////////////////////////////////////////////////
// Using

use shrev::{Event, EventChannel, ReaderId};

//////////////////////////////////////////////////
// Definition

#[derive(Debug)]
pub struct Events<T: Event> {
    channel: EventChannel<T>,
    queue: Vec<(f32, T)>,
}

//////////////////////////////////////////////////
// Implementation

impl<T: Event> Events<T> {
    pub fn new() -> Events<T> {
        Events {
            channel: EventChannel::new(),
            queue: Vec::new(),
        }
    }

    pub fn register(&mut self) -> ReaderId<T> {
        self.channel.register_reader()
    }

    pub fn write(&mut self, event: T) {
        self.channel.single_write(event);
    }

    pub fn write_delayed(&mut self, event: T, delay: f32) {
        self.queue.push((delay, event));
        self.queue.sort_by(|(a, _), (b, _)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal).reverse());
    }

    pub fn update_delayed(&mut self, elapsed_time: f32) {
        // update delay
        self.queue.iter_mut().for_each(|(time, _)| {
            *time -= elapsed_time;
        });

        // write events from queue to channel
        while self.queue.last().map(|(time, _)| *time <= 0.0).unwrap_or(false) {
            self.channel.single_write(self.queue.pop().unwrap().1);
        }
    }

    pub fn read(&self, reader: &mut ReaderId<T>) -> Vec<&T> {
        self.channel.read(reader).collect()
    }

    pub fn read_opt(&self, reader: &mut Option<ReaderId<T>>) -> Vec<&T> {
        if let Some(reader) = reader.as_mut() {
            self.read(reader)
        } else {
            Vec::new()
        }
    }

    pub fn ignore(&self, reader: &mut ReaderId<T>) {
        self.channel.read(reader);
    }
}

impl<T: Event + Clone> Default for Events<T> {
    fn default() -> Self {
        Self::new()
    }
}
