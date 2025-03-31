use std::time::Instant;

use chrono::NaiveDate;
use futures::{StreamExt, stream::FuturesUnordered};
use reqwest::Response;
use select::{
    document::Document,
    predicate::{Attr, Name},
};
use tokio::sync::mpsc;
use tracing::{debug, info, instrument, trace, warn};

use crate::Resource;

#[instrument(skip(document))]
fn extract_title_and_description(document: &Document) -> (Option<String>, Option<String>) {
    let mut title = None;
    let mut content = None;

    if let Some(the_title) = document.find(Name("title")).next() {
        title = Some(the_title.text());
    }
    if let Some(description) = document.find(Attr("name", "description")).next() {
        content = description.attr("content").map(ToString::to_string);
    }
    (title, content)
}

async fn page_details(
    (response, timestamp): (Response, NaiveDate),
) -> Result<Resource, reqwest::Error> {
    let url = response.url().clone();
    let start = Instant::now();
    let text = response.text().await?;
    trace!(took = ?start.elapsed(), "extracted text");
    let before_thread = Instant::now();
    let (title, description) = tokio::task::spawn_blocking(move || {
        let document = Document::from(text.as_str());
        extract_title_and_description(&document)
    })
    .await
    .unwrap();
    debug!(
        %url,
        took = ?before_thread.elapsed(),
        "Finished processing",
    );
    let resource = Resource {
        url,
        title: title.unwrap_or_default(),
        description: description.unwrap_or_default(),
        timestamp,
    };
    Ok(resource)
}

#[instrument(skip_all)]
pub async fn processor(
    mut pages: mpsc::Receiver<(Response, NaiveDate)>,
    forward: mpsc::Sender<Resource>,
    limit: usize,
) {
    let mut work = FuturesUnordered::new();

    loop {
        let in_progress = work.len();
        tokio::select! {
            biased;

            Some(page) = pages.recv(), if in_progress < limit => {
                work.push(page_details(page));
            },
            Some(resource) = work.next(), if in_progress > 0 => {
                match resource {
                    Ok(resource) => {
                        if let Err(error) = forward.send(resource).await {
                            warn!(%error, "Page processor cannot forward, shutting down");
                            break;
                        }
                    }
                    Err(error) => {
                        warn!(%error, "Failed to process page");
                    }
                }
            },
            else => { break }
        }
    }
    info!("Page processing finished");
}
