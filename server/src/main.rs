use anyhow::Error;
use rss::Channel;

#[tokio::main]
pub async fn main() {
    let feed_urls = vec![
        "https://www.nasa.gov/rss/dyn/breaking_news.rss".to_string(),
        "https://rss.art19.com/apology-line".to_string(),
    ];

    let channel_futs: Vec<_> = feed_urls
        .iter()
        .map(|feed_url| get_channel(feed_url.clone()))
        .collect();

    for channel_fut in channel_futs {
        output_channel(channel_fut.await.unwrap());
    }
    println!("after loop");
}

async fn get_channel(feed_url: String) -> Result<Channel, Error> {
    println!("about to start with {}", feed_url);
    let content = reqwest::get(&feed_url).await?.bytes().await?;
    println!("done with {}", feed_url);

    let channel = Channel::read_from(&content[..])?;
    Ok(channel)
}

pub fn output_channel(channel: Channel) {
    println!("Channel: {}", channel.title);
}
