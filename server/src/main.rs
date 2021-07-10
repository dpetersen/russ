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
    let (tx, mut rx) = mpsc::channel::<Channel>(6);

    let fetching = fetcher::fetch_all(feed_urls, tx);
    let outputting = async {
        while let Some(channel) = rx.recv().await {
            output_channel(channel);
        }
    };

    tokio::join!(fetching, outputting);
}

pub fn output_channel(channel: Channel) {
    println!("Channel: {}", channel.title);
}
