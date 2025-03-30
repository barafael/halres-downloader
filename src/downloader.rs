use chrono::NaiveDate;
use futures::{StreamExt, stream::FuturesUnordered};
use reqwest::Response;
use tokio::sync::mpsc;
use tracing::{debug, info, warn};

use crate::Record;

async fn download_page(record: Record) -> Result<(Response, NaiveDate), reqwest::Error> {
    let url = record.url;
    debug!(%url, "Downloading page");
    let timestamp = record.timestamp;
    let response = reqwest::get(url).await?;
    Ok((response, timestamp))
}

pub async fn download_pages(
    mut pages: mpsc::Receiver<Record>,
    forward: mpsc::Sender<(Response, NaiveDate)>,
) {
    let mut work = FuturesUnordered::new();

    loop {
        tokio::select! {
            Some(record) = pages.recv() => {
                work.push(download_page(record));
            },
            Some(result) = work.next(), if !work.is_empty() => {
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
