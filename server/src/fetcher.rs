use anyhow::Error;
use rss::Channel;
use tokio::sync::mpsc;

pub async fn fetch_all(feed_urls: Vec<String>, channels: mpsc::Sender<Channel>) {
    let handles: Vec<_> = feed_urls
        .into_iter()
        .map(|feed_url| tokio::spawn(get_channel(feed_url, channels.clone())))
        .collect();

    for handle in handles {
        match handle.await {
            Err(e) => error!("failed completing fetch task: {}", e),
            Ok(res) => match res {
                Err(e) => error!("error fetching feed: {}", e),
                Ok(feed_url) => info!("feed fetched: {}", feed_url),
            },
        }
    }
}

async fn get_channel(feed_url: String, tx: mpsc::Sender<Channel>) -> Result<String, Error> {
    let content = reqwest::get(&feed_url).await?.bytes().await?;

    let channel = Channel::read_from(&content[..])?;
    tx.send(channel).await?;
    Ok(feed_url)
}
