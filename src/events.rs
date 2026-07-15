use std::{any::TypeId, collections::HashMap, time};

use crate::{
    buffers::{Buffer, Cursor, ErasedBuffer},
    config::WindowMode,
    renderer::PresentMode,
};

pub trait Event: 'static {}

pub struct EventBus {
    buffers: HashMap<TypeId, Box<dyn ErasedBuffer>>,
}

impl EventBus {
    pub(crate) fn new() -> Self {
        Self {
            buffers: HashMap::new(),
        }
    }

    pub fn cursor<T: Event>(&self) -> Option<Cursor<T>> {
        self.buffers
            .get(&TypeId::of::<T>())
            .and_then(|b| b.as_any().downcast_ref::<Buffer<T>>())
            .map(|buf| buf.cursor())
    }

    pub fn publish<T: Event>(&mut self, v: T) {
        self.buffers
            .entry(TypeId::of::<T>())
            .or_insert_with(|| Box::new(Buffer::<T>::new()))
            .as_any_mut()
            .downcast_mut::<Buffer<T>>()
            .expect("TypeId keyed the wrong buffer")
            .publish(v)
    }

    pub fn read<'a, T: Event>(&'a self, cursor: &mut Cursor<T>) -> impl Iterator<Item = &'a T> {
        self.buffers
            .get(&TypeId::of::<T>())
            .and_then(|b| b.as_any().downcast_ref::<Buffer<T>>())
            .map(|buf| buf.read(cursor))
            .into_iter()
            .flatten()
    }

    pub(crate) fn swap_all(&mut self) {
        for buf in self.buffers.values_mut() {
            buf.swap();
        }
    }
}

// ----------------------------------------------------------------------------

#[derive(Debug, Clone, Copy)]
pub struct Exiting;

impl Event for Exiting {}

#[derive(Debug, Clone, Copy)]
pub struct MemoryWarning;

impl Event for MemoryWarning {}

#[derive(Debug, Clone, Copy)]
pub struct PresentModeChanged {
    pub mode: PresentMode,
}

impl Event for PresentModeChanged {}

#[derive(Debug, Clone, Copy)]
pub struct Resumed;

impl Event for Resumed {}

#[derive(Debug, Clone, Copy)]
pub struct Suspended;

impl Event for Suspended {}

#[derive(Debug, Clone, Copy)]
pub struct TickFrequencyChanged {
    pub freq: time::Duration,
}

impl Event for TickFrequencyChanged {}

#[derive(Debug, Clone, Copy)]
pub struct WindowFocused {
    pub focused: bool,
}

impl Event for WindowFocused {}

#[derive(Debug, Clone, Copy)]
pub struct WindowModeChanged {
    pub mode: WindowMode,
}

impl Event for WindowModeChanged {}

#[derive(Debug, Clone, Copy)]
pub struct WindowOccluded {
    pub occluded: bool,
}

impl Event for WindowOccluded {}

#[derive(Debug, Clone, Copy)]
pub struct WindowScaleChanged {
    pub scale_factor: f64,
}

impl Event for WindowScaleChanged {}
