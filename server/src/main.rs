use anyhow::Error;
use rss::Channel;
use tokio::sync::mpsc;

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

    let fetching = fetcher::fetch_all(feed_urls, channel_tx, error_tx);
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

    tokio::join!(fetching, outputting_channels, outputting_errors);
}

pub fn output_channel(channel: Channel) {
    println!("Channel: {}", channel.title);
}
