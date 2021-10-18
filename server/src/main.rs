use anyhow::Error;
use rss::Channel;
use signal_hook::{consts::SIGINT, consts::SIGTERM, iterator::Signals};
use tokio::sync::{mpsc, oneshot};

#[macro_use]
extern crate log;

mod fetcher;
mod persistence;

#[tokio::main]
pub async fn main() {
    env_logger::init();

    // TODO pull this from the config
    let feed_urls = vec![
        "https://www.nasa.gov/rss/dyn/breaking_news.rss".to_string(),
        "https://rss.art19.com/apology-line".to_string(),
        "https://example.com/bad".to_string(),
    ];
    let (feed_channel_tx, mut feed_channel_rx) = mpsc::channel::<(String, Channel)>(6);
    let (fetch_error_tx, mut fetch_error_rx) = mpsc::channel::<Error>(6);
    let (quit_tx, quit_rx) = oneshot::channel();
    // TODO path somewhere in an appropriate home path
    let mut database =
        match persistence::FileDatabase::new_for_path(std::path::Path::new("database.json")) {
            Ok(d) => d,
            Err(e) => {
                error!("error opening database: {}", e);
                std::process::exit(1);
            }
        };

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

    let fetching = fetcher::cancellable_periodic_fetch(
        feed_urls,
        fetcher::Results::new(feed_channel_tx, fetch_error_tx),
        quit_rx,
    );
    let outputting_channels = async {
        while let Some((feed_url, channel)) = feed_channel_rx.recv().await {
            if let Err(e) = database.persist_channel(feed_url, &channel) {
                error!("persisting channel '{}': {}", channel.title, e);
            }
        }
    };
    let outputting_errors = async {
        while let Some(error) = fetch_error_rx.recv().await {
            error!("error fetching feed: {}", error);
        }
    };

    let (fetching_result, _, _) = tokio::join!(fetching, outputting_channels, outputting_errors);
    match fetching_result {
        Err(e) => warn!("problem gracefully shutting down fetcher: {}", e),
        Ok(()) => info!("gracefully quit fetching"),
    }
}
