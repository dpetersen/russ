use anyhow::Error;
use rss::Channel;

#[tokio::main]
pub async fn main() {
    let feed_urls = vec![
        "https://www.nasa.gov/rss/dyn/breaking_news.rss".to_string(),
        "https://rss.art19.com/apology-line".to_string(),
    ];

    let handles: Vec<_> = feed_urls
        .into_iter()
        .map(|feed_url| tokio::spawn(get_channel(feed_url)))
        .collect();

    for handle in handles {
        match handle.await {
            Err(e) => panic!("Fetching feed: {}", e),
            Ok(_) => println!("success"),
        }
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
