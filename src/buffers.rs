use std::{marker::PhantomData, mem};

use crate::macros::erased_downcast;

pub(crate) struct Buffer<T> {
    count: u64,
    current: Vec<T>,
    oldest_id: u64,
    prev: Vec<T>,
}

erased_downcast!(Buffer, {
    swap() -> ();
});

impl<T> Buffer<T> {
    pub(crate) fn new() -> Self {
        Self {
            count: 0,
            current: Vec::new(),
            oldest_id: 0,
            prev: Vec::new(),
        }
    }

    pub(crate) fn cursor(&self) -> Cursor<T> {
        Cursor::from_offset(self.oldest_id)
    }

    pub(crate) fn publish(&mut self, v: T) {
        self.count += 1;
        self.current.push(v);
    }

    pub(crate) fn read(&self, cursor: &mut Cursor<T>) -> impl Iterator<Item = &T> {
        let offset = (cursor.last_read.max(self.oldest_id) - self.oldest_id) as usize;
        cursor.last_read = self.count;

        let (p, c) = if offset < self.prev.len() {
            (&self.prev[offset..], &self.current[..])
        } else {
            (&[][..], &self.current[offset - self.prev.len()..])
        };

        p.iter().chain(c.iter())
    }

    pub(crate) fn swap(&mut self) {
        self.oldest_id += self.prev.len() as u64;
        mem::swap(&mut self.current, &mut self.prev);
        self.current.clear();
    }
}

pub struct Cursor<T> {
    last_read: u64,
    _marker: PhantomData<fn() -> T>,
}

impl<T> Cursor<T> {
    pub fn new() -> Self {
        Self {
            last_read: 0,
            _marker: PhantomData,
        }
    }

    fn from_offset(offset: u64) -> Self {
        let mut cursor = Cursor::new();
        cursor.last_read = offset;
        cursor
    }
}

impl<T> Default for Cursor<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn collect<T: Copy>(buffer: &Buffer<T>, cursor: &mut Cursor<T>) -> Vec<T> {
        buffer.read(cursor).copied().collect()
    }

    #[test]
    fn empty_buffer_reads_nothing() {
        let buffer = Buffer::<u32>::new();
        let mut cursor = buffer.cursor();
        assert_eq!(collect(&buffer, &mut cursor), Vec::<u32>::new());
    }

    #[test]
    fn re_read_after_publish_returns_only_new() {
        let mut buffer = Buffer::<u32>::new();
        buffer.publish(1);
        buffer.publish(2);

        let mut cursor = buffer.cursor();
        assert_eq!(collect(&buffer, &mut cursor), vec![1, 2]);

        buffer.publish(3);
        assert_eq!(collect(&buffer, &mut cursor), vec![3]);
    }

    #[test]
    fn re_read_without_publish_returns_nothing() {
        let mut buffer = Buffer::<u32>::new();
        buffer.publish(1);

        let mut cursor = buffer.cursor();
        assert_eq!(collect(&buffer, &mut cursor), vec![1]);
        assert_eq!(collect(&buffer, &mut cursor), Vec::<u32>::new());
    }

    #[test]
    fn reads_across_prev_and_current() {
        let mut buffer = Buffer::<u32>::new();
        buffer.publish(1);
        buffer.publish(2);
        buffer.swap();
        buffer.publish(3);

        let mut cursor = buffer.cursor();
        assert_eq!(collect(&buffer, &mut cursor), vec![1, 2, 3]);
    }

    #[test]
    fn partial_cursor_starts_mid_prev() {
        let mut buffer = Buffer::<u32>::new();
        buffer.publish(1);
        buffer.publish(2);

        let mut cursor = buffer.cursor();
        assert_eq!(collect(&buffer, &mut cursor), vec![1, 2]);

        buffer.swap();
        buffer.publish(3);
        assert_eq!(collect(&buffer, &mut cursor), vec![3]);
    }

    #[test]
    fn lagging_cursor_after_double_swap_reads_available() {
        let mut buffer = Buffer::<u32>::new();
        buffer.publish(1);
        buffer.publish(2);

        let mut cursor = buffer.cursor();

        buffer.swap();
        buffer.publish(3);
        buffer.swap();
        buffer.publish(4);

        assert_eq!(collect(&buffer, &mut cursor), vec![3, 4]);
    }

    #[test]
    fn swap_advances_oldest_id_and_drops_events() {
        let mut buffer = Buffer::<u32>::new();
        buffer.publish(1);
        buffer.publish(2);
        buffer.swap();
        buffer.publish(3);
        buffer.swap();

        assert_eq!(buffer.oldest_id, 2);

        let mut cursor = buffer.cursor();
        assert_eq!(collect(&buffer, &mut cursor), vec![3]);
    }

    #[test]
    fn swap_with_empty_current_keeps_oldest_id() {
        let mut buffer = Buffer::<u32>::new();
        buffer.swap();
        assert_eq!(buffer.oldest_id, 0);

        buffer.publish(1);
        let mut cursor = buffer.cursor();
        assert_eq!(collect(&buffer, &mut cursor), vec![1]);
    }
}
