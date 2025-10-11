use anyhow::{Error, anyhow};
use crossbeam_channel::{Receiver, SendError, Sender, bounded};

use crate::utils::batched_data::BatchedData;

pub struct ChannelPair<T> {
    pub tx: crossbeam_channel::Sender<T>,
    pub rx: crossbeam_channel::Receiver<T>,
}

impl<T> ChannelPair<T> {
    pub fn new_full(data_init: impl Fn() -> T, capacity: usize) -> Result<ChannelPair<T>, Error> {
        let (tx, rx) = crossbeam_channel::bounded(capacity);

        for _ in 0..capacity {
            match tx.send(data_init()) {
                Ok(_) => {}
                Err(err) => Err(anyhow!("{err:?}"))?,
            }
        }

        Ok(Self { tx, rx })
    }

    pub fn into_sender_receiver_tup(self) -> (Sender<T>, Receiver<T>) {
        (self.tx, self.rx)
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
    data: ChannelPair<BatchedData<T>>,
    buffer: ChannelPair<BatchedData<T>>,
}

impl<T> BatchedChannel<T> {
    pub fn new(
        data_init: impl Fn() -> T,
        data_batch_size: usize,
        channel_capacity: usize,
    ) -> Result<Self, Error> {
        let (tx_buf, rx_buf) = bounded(channel_capacity);
        let (tx_data, rx_data) = bounded(channel_capacity);

        for i in 0..channel_capacity {
            match tx_buf.send(BatchedData::from_vec(
                (0..data_batch_size).map(|_| data_init()).collect(),
            )) {
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



#[cfg(test)]
mod tests {
    use super::*;

    fn test_init() -> Result<(), Box<dyn std::error::Error>> {
        let bc = BatchedChannel::new(|| String::new(), 1024, 1024)?;
        
        Ok(())
    }
}
