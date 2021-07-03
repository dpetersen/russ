use rss::Channel;
use std::error::Error;

#[tokio::main]
pub async fn main() {
    let channel = get_channel().await.unwrap();
    println!(
        "Got channel {}\n{}\n---------------",
        channel.title, channel.description
    );
    for item in channel.items {
        println!(
            "Title: {}\nDescription:\n{}\n---------------",
            item.title.unwrap_or("N/A".to_string()),
            item.description.unwrap_or("N/A".to_string()),
        );
    }
}

async fn get_channel() -> Result<Channel, Box<dyn Error>> {
    let content = reqwest::get("https://www.nasa.gov/rss/dyn/breaking_news.rss")
        .await?
        .bytes()
        .await?;
    let channel = Channel::read_from(&content[..])?;
    Ok(channel)
}
