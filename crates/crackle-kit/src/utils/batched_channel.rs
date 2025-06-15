use anyhow::{Error, anyhow};
use crossbeam_channel::{Receiver, SendError, Sender, bounded};

pub struct ChannelPair<T> {
    pub tx: Sender<T>,
    pub rx: Receiver<T>,
}

impl<T> ChannelPair<T> {
    fn new_full(data_init: impl Fn() -> T, capacity: usize) -> Result<ChannelPair<T>, Error> {
        let (tx, rx) = bounded(capacity);

        for _ in 0..capacity {
            match tx.send(data_init()) {
                Ok(_) => {}
                Err(err) => Err(anyhow!("{err:?}"))?,
            }
        }

        Ok(Self { tx, rx })
    }
}

/*
Needed
1. check data is empty. (we use batch so items in back may be empty.) -> type T should handle this.
2. make filled channel. -> Channel Pair
3.
*/

///
/// ```
/// let bc = BatchedChannel::default();
///
/// if Ok(data) = bc.data.rx.try_recv() {
///     // check data is filled.
///     
///
/// }
///
/// if Ok(buffer) = bc.buffer.rx.try_recv() {
///
/// }
/// ```
pub struct BatchedChannel<T> {
    data: ChannelPair<Vec<T>>,
    buffer: ChannelPair<Vec<T>>,
}

impl<T> BatchedChannel<T> {
    fn new(data_init: impl Fn() -> T, batch_size: usize, pool_size: usize) -> Result<Self, Error> {
        let (tx_buf, rx_buf) = bounded(pool_size);
        let (tx_data, rx_data) = bounded(pool_size);

        for i in 0..pool_size {
            match tx_buf.send((0..batch_size).map(|_| data_init()).collect()) {
                Ok(_) => {}
                Err(err) => Err(anyhow!("{err:?}"))?,
            }
        }

        Ok(Self {
            data: ChannelPair {
                rx: rx_data,
                tx: tx_data,
            },
            buffer: ChannelPair {
                rx: rx_buf,
                tx: tx_buf,
            },
        })
    }
}

pub trait BatchedChannelData {
    fn clear(&mut self);
}
