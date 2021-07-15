use anyhow::{bail, Error};
use rss::Channel;
use tokio::sync::{mpsc, oneshot};
use tokio::time::{self, Duration};

const POLL_INTERVAL_SECONDS: u64 = 5;

pub async fn cancellable_periodic_fetch(
    feed_urls: Vec<String>,
    channels: mpsc::Sender<Channel>,
    errors: mpsc::Sender<Error>,
    quit: oneshot::Receiver<()>,
) -> Result<(), Error> {
    tokio::select! {
        _ = periodically_fetch(feed_urls, channels, errors) => {
            bail!("fetcher has unexpectely quit");
        },
        _ = quit => {
            info!("asked to quit fetching gracefully");
            Ok(())
        },
    }
}

async fn periodically_fetch(
    feed_urls: Vec<String>,
    channels: mpsc::Sender<Channel>,
    errors: mpsc::Sender<Error>,
) {
    let mut timer = time::interval(Duration::from_secs(POLL_INTERVAL_SECONDS));
    timer.set_missed_tick_behavior(time::MissedTickBehavior::Skip);

    loop {
        timer.tick().await;
        fetch_all(feed_urls.clone(), &channels, &errors).await;
    }
}

pub async fn fetch_all(
    feed_urls: Vec<String>,
    channels: &mpsc::Sender<Channel>,
    errors: &mpsc::Sender<Error>,
) {
    let handles: Vec<_> = feed_urls
        .into_iter()
        .map(|feed_url| tokio::spawn(get_channel(feed_url, channels.clone())))
        .collect();

    for handle in handles {
        match handle.await {
            Err(e) => error!("failed completing fetch task: {}", e),
            Ok(res) => match res {
                Err(e) => {
                    if let Err(send_err) = errors.send(e).await {
                        error!("sending fetch error: {}", send_err);
                    }
                }
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

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn test_fetch_all() {
        let mock_server = MockServer::start().await;
        let nasa_response = ResponseTemplate::new(200).set_body_string(r#"
            <?xml version="1.0" encoding="utf-8" ?>
            <rss version="2.0" xml:base="http://www.nasa.gov/" xmlns:atom="http://www.w3.org/2005/Atom" xmlns:dc="http://purl.org/dc/elements/1.1/" xmlns:itunes="http://www.itunes.com/dtds/podcast-1.0.dtd" xmlns:media="http://search.yahoo.com/mrss/">
                <channel>
                    <title>NASA Breaking News</title>
                    <description>A RSS news feed containing the latest NASA news articles and press releases.</description>
                    <link>http://www.nasa.gov/</link>
                    <atom:link rel="self" href="http://www.nasa.gov/rss/dyn/breaking_news.rss" />
                    <language>en-us</language>
                    <docs>http://blogs.harvard.edu/tech/rss</docs>
                    <item>
                        <title>NC, Wisconsin, NY Students to Hear from Astronauts on Space Station</title>
                        <link>http://www.nasa.gov/press-release/nc-wisconsin-ny-students-to-hear-from-astronauts-on-space-station</link>
                        <description>Students from three states will hear from astronauts from three different countries aboard the International Space Station next week.</description>
                        <enclosure url="http://www.nasa.gov/sites/default/files/styles/1x1_cardfeed/public/thumbnails/image/iss065e084898_0.jpg?itok=HzmCp_DJ" length="6451240" type="image/jpeg" />
                        <guid isPermaLink="false">http://www.nasa.gov/press-release/nc-wisconsin-ny-students-to-hear-from-astronauts-on-space-station</guid>
                        <pubDate>Fri, 09 Jul 2021 17:08 EDT</pubDate>
                        <source url="http://www.nasa.gov/rss/dyn/breaking_news.rss">NASA Breaking News</source>
                        <dc:identifier>472446</dc:identifier>
                    </item>
                </channel>
            </rss>
        "#);
        Mock::given(method("GET"))
            .and(path("/feed"))
            .respond_with(nasa_response)
            .mount(&mock_server)
            .await;
        Mock::given(method("GET"))
            .and(path("/bad"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&mock_server)
            .await;

        let (channel_tx, mut channel_rx) = mpsc::channel::<Channel>(6);
        let (error_tx, mut error_rx) = mpsc::channel::<Error>(6);
        let (quit_tx, quit_rx) = oneshot::channel();
        let fetching = cancellable_periodic_fetch(
            vec![
                format!("{}/feed", &mock_server.uri()),
                format!("{}/bad", &mock_server.uri()),
            ],
            channel_tx,
            error_tx,
            quit_rx,
        );
        let waiter = async {
            // Not in love with this sleep. I should try and have something that triggers when both
            // endpoints have been hit, but I'll bet that will be tough with lifetimes... But hey,
            // at least I'm testing graceful termination.
            tokio::time::sleep(Duration::from_millis(10)).await;
            quit_tx.send(()).unwrap();
        };

        let mut channels: Vec<Channel> = Vec::new();
        let reading_channels = async {
            while let Some(channel) = channel_rx.recv().await {
                channels.push(channel);
            }
        };
        let mut errors: Vec<Error> = Vec::new();
        let reading_errors = async {
            while let Some(error) = error_rx.recv().await {
                errors.push(error);
            }
        };
        let (fetching_result, _, _, _) =
            tokio::join!(fetching, reading_channels, reading_errors, waiter);
        assert!(fetching_result.is_ok());

        assert_eq!(1, channels.len());
        assert_eq!("NASA Breaking News", channels.first().unwrap().title);

        assert_eq!(1, errors.len());
        assert_eq!(
            "reached end of input without finding a complete channel",
            errors.first().unwrap().to_string()
        );
    }
}
