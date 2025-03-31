use std::time::Duration;

use chrono::NaiveDate;
use futures::{StreamExt, stream::FuturesUnordered};
use reqwest::Response;
use tokio::sync::mpsc;
use tracing::{debug, info, instrument, warn};

use crate::Record;

static USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"),);

async fn download_page(record: Record) -> Result<(Response, NaiveDate), reqwest::Error> {
    let url = record.url;
    debug!(%url, "Downloading page");
    let timestamp = record.timestamp;
    let client = reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .connect_timeout(Duration::from_secs(20))
        .timeout(Duration::from_secs(30))
        .build()?;
    let response = client.get(url).send().await?;
    Ok((response, timestamp))
}

#[instrument(skip_all)]
pub async fn download_pages(
    mut pages: mpsc::Receiver<Record>,
    forward: mpsc::Sender<(Response, NaiveDate)>,
    limit: usize,
) {
    let mut work = FuturesUnordered::new();

    loop {
        let in_progress = work.len();
        tokio::select! {
            biased;

            Some(record) = pages.recv(), if in_progress < limit => {
                work.push(download_page(record));
            },
            Some(result) = work.next(), if in_progress > 0 => {
                match result {
                    Ok(result)=> {
                        if let Err(error) = forward.send(result).await {
                            warn!(%error, "Page downloader cannot forward, shutting down");
                            break;
                        }
                    }
                    Err(error) => {
                        warn!(%error, "Failed to download page");
                    }
                }
            },
            else => { break }
        }
    }
    info!("Page download finished");
}
