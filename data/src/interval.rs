use once_cell::sync::Lazy;
use std::{ops::Deref, time::Duration};
use tokio::{
    sync::broadcast::{channel, Sender},
    time,
};

const CAPACITY: usize = 100;
const WATCH_INTERVAL: i64 = 30000;

pub static WATCH_INTERVAL_TX: Lazy<Sender<Tick>> = Lazy::new(|| {
    let (interval_tx, interval_rx) = channel::<Tick>(CAPACITY);
    drop(interval_rx);
    interval_tx
});

#[derive(Clone, Copy, Debug)]
pub struct Tick;

#[inline]
pub async fn send_tick() {
    let tx = WATCH_INTERVAL_TX.deref();
    send_tick_with_interval(tx, WATCH_INTERVAL).await
}

async fn send_tick_with_interval(interval_tx: &Sender<Tick>, interval: i64) {
    let now = chrono::Utc::now().timestamp_millis();
    let elapse = interval - (now - now / interval * interval);
    log::info!("millisecond waited for sending Tick: {}", elapse);
    let start = time::Instant::now()
        .checked_add(Duration::from_millis(elapse as u64))
        .expect("failed to construct an Instant");
    let mut interval = time::interval_at(start, Duration::from_millis(interval as u64));
    loop {
        let _ = interval.tick().await;
        let _ = interval_tx.send(Tick);
    }
}
