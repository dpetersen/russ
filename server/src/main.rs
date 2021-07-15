use anyhow::Error;
use rss::Channel;
use signal_hook::{consts::SIGINT, consts::SIGTERM, iterator::Signals};
use tokio::sync::{mpsc, oneshot};

#[macro_use]
extern crate log;

mod fetcher;

#[tokio::main]
pub async fn main() {
    env_logger::init();

    let feed_urls = vec![
        "https://www.nasa.gov/rss/dyn/breaking_news.rss".to_string(),
        "https://rss.art19.com/apology-line".to_string(),
        "https://example.com/bad".to_string(),
    ];
    let (channel_tx, mut channel_rx) = mpsc::channel::<Channel>(6);
    let (error_tx, mut error_rx) = mpsc::channel::<Error>(6);
    let (quit_tx, quit_rx) = oneshot::channel();

    let mut signals = match Signals::new(&[SIGINT, SIGTERM]) {
        Ok(s) => s,
        Err(e) => panic!("unable to set up quit signal handler: {}", e),
    };
    std::thread::spawn(move || {
        for sig in signals.forever() {
            debug!("received signal {:?}", sig);
            break;
        }
        if let Err(_) = quit_tx.send(()) {
            error!("problem sending quit signal to the refresher");
        };
    });

    let fetching = fetcher::cancellable_periodic_fetch(feed_urls, channel_tx, error_tx, quit_rx);
    let outputting_channels = async {
        while let Some(channel) = channel_rx.recv().await {
            output_channel(channel);
        }
    };
    let outputting_errors = async {
        while let Some(error) = error_rx.recv().await {
            error!("error fetching feed: {}", error);
        }
    };

    let (fetching_result, _, _) = tokio::join!(fetching, outputting_channels, outputting_errors);
    match fetching_result {
        Err(e) => warn!("problem gracefully shutting down fetcher: {}", e),
        Ok(()) => info!("successfully quit fetching gracefully"),
    }
}

pub fn output_channel(channel: Channel) {
    println!("Channel: {}", channel.title);
}
