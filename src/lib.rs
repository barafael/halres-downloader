use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use url::Url;

mod downloader;
mod processor;

#[derive(Debug, Serialize, Deserialize)]
pub struct Resource {
    pub url: Url,
    pub title: String,
    pub description: String,
    pub timestamp: NaiveDate,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Record {
    pub timestamp: NaiveDate,
    pub url: Url,
}

#[must_use = "Use the returned sender to send records, dropping it when done, and the receiver to receive resources when they become available."]
pub fn run(
    channel_size: usize,
    concurrency_limit: usize,
) -> (mpsc::Sender<Record>, mpsc::Receiver<Resource>) {
    let (pages_tx, pages_rx) = mpsc::channel(channel_size);
    let (process_tx, process_rx) = mpsc::channel(channel_size);
    let (resource_tx, resource_rx) = mpsc::channel(channel_size);

    let _downloader = tokio::spawn(downloader::download_pages(
        pages_rx,
        process_tx,
        concurrency_limit,
    ));
    let _processor = tokio::spawn(processor::processor(
        process_rx,
        resource_tx,
        concurrency_limit,
    ));
    (pages_tx, resource_rx)
}

pub async fn download_pages(
    channel_size: usize,
    concurrency_limit: usize,
    urls: Vec<Record>,
) -> Vec<Resource> {
    let (pages_tx, mut resource_rx) = run(channel_size, concurrency_limit);

    let collector = tokio::spawn(async move {
        let mut resources = Vec::new();
        while let Some(resource) = resource_rx.recv().await {
            resources.push(resource);
        }
        resources
    });

    for record in urls {
        pages_tx.send(record).await.expect("Failed to send record");
    }
    // let the downloader know there won't be any more incoming records for it.
    drop(pages_tx);
    // after this point, the downloader and processor shutdown on their own.

    collector.await.expect("Failed to collect resources")
}
