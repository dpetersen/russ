use anyhow::Error;
use rss::Channel;
use tokio::sync::mpsc;

#[tokio::main]
pub async fn main() {
    let feed_urls = vec![
        "https://www.nasa.gov/rss/dyn/breaking_news.rss".to_string(),
        "https://rss.art19.com/apology-line".to_string(),
    ];

    let (tx, mut rx) = mpsc::channel::<Channel>(6);

    let handles: Vec<_> = feed_urls
        .into_iter()
        .map(|feed_url| tokio::spawn(get_channel(feed_url, tx.clone())))
        .collect();
    // It's not clear to me why this isn't dropped by Rust, since it's no longer used beyond this
    // point. But if I don't drop this, rx never closes.
    drop(tx);

    for handle in handles {
        match handle.await {
            Err(e) => eprintln!("Fetching feed: {}", e),
            Ok(_) => println!("success"),
        }
    }

    while let Some(channel) = rx.recv().await {
        output_channel(channel);
    }

    println!("after loop");
}

async fn get_channel(feed_url: String, tx: mpsc::Sender<Channel>) -> Result<(), Error> {
    println!("about to start with {}", feed_url);
    let content = reqwest::get(&feed_url).await?.bytes().await?;
    println!("done with {}", feed_url);

    let channel = Channel::read_from(&content[..])?;
    tx.send(channel).await?;
    Ok(())
}

pub fn output_channel(channel: Channel) {
    println!("Channel: {}", channel.title);
}
