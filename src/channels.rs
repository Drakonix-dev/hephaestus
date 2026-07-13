use std::sync::mpsc::{self, Receiver, Sender};

pub(crate) fn channel<T>() -> (Writer<T>, Reader<T>) {
    let (tx, rx) = mpsc::channel();
    (Writer { tx }, Reader { rx })
}

#[derive(Clone)]
pub(crate) struct Writer<T> {
    tx: Sender<T>,
}

impl<T> Writer<T> {
    pub(crate) fn push(&self, d: T) {
        self.tx
            .send(d)
            .expect("pushed data after queue reader dropped");
    }
}

pub(crate) struct Reader<T> {
    rx: Receiver<T>,
}

impl<T> Reader<T> {
    pub(crate) fn drain(&self) -> Vec<T> {
        let mut data = Vec::new();

        while let Ok(d) = self.rx.try_recv() {
            data.push(d);
        }

        data
    }
}

macro_rules! define_channel {
    ($name:ident, $data:ty) => {
        use paste::paste;

        paste! {
            pub(crate) type [< $name Reader>] = crate::channels::Reader<$data>;
            pub(crate) type [< $name Writer>] = crate::channels::Writer<$data>;

            pub(crate) fn [< $name:snake _channel>]() -> ([< $name Writer >], [< $name Reader >]) {
                crate::channels::channel()
            }
        }
    };
}

pub(crate) use define_channel;
