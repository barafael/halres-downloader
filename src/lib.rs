use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use url::Url;

mod downloader;
mod processor;

#[derive(Debug, Serialize, Deserialize)]
pub struct Resource {
    url: Url,
    content: String,
    title: String,
    timestamp: NaiveDate,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Record {
    timestamp: NaiveDate,
    url: Url,
}

pub async fn run() -> (mpsc::Sender<Record>, mpsc::Receiver<Resource>) {
    let (pages_tx, pages_rx) = mpsc::channel(64);
    let (process_tx, process_rx) = mpsc::channel(64);
    let (resource_tx, resource_rx) = mpsc::channel(64);

    let _downloader = tokio::spawn(downloader::download_pages(pages_rx, process_tx));
    let _processor = tokio::spawn(processor::processor(process_rx, resource_tx));
    (pages_tx, resource_rx)
}
