use std::sync::mpsc::{self, Sender};
use std::thread;

use hal::display::{EpaperDisplay, Redraw, Refreshed};
use hal::Fault;

use crate::latest::Latest;
use crate::screen::{self, Legend, Page};

pub(crate) type LastRefresh = Option<Result<Refreshed, Fault>>;

/// Draws pages on a thread of its own, since a refresh blocks for up to seconds. Pages
/// that arrive during a refresh are skipped, all but the latest.
pub(crate) struct Painter {
    pages: Sender<Page>,
    last_refresh: Latest<LastRefresh>,
}

impl Painter {
    pub fn spawn(mut display: impl EpaperDisplay + Send + 'static, legend: Legend) -> Result<Self, Fault> {
        let (pages, queued) = mpsc::channel::<Page>();
        let last_refresh = Latest::new(None);
        let published = last_refresh.clone();
        thread::Builder::new()
            .name("painter".into())
            .stack_size(16 * 1024)
            .spawn(move || {
                let mut shown: Option<Page> = None;
                while let Ok(mut page) = queued.recv() {
                    while let Ok(newer) = queued.try_recv() {
                        page = newer;
                    }
                    let redraw = match &shown {
                        Some(previous) if previous.is_same_kind(&page) => Redraw::Changes,
                        _ => Redraw::Whole,
                    };
                    let refreshed = display.show(&screen::draw(&page, &legend), redraw);
                    match &refreshed {
                        Ok(r) => log::debug!("panel: {r:?}"),
                        Err(fault) => log::warn!("panel: {fault}"),
                    }
                    published.set(Some(refreshed));
                    shown = Some(page);
                }
            })
            .map_err(Fault::new)?;
        Ok(Self { pages, last_refresh })
    }

    pub fn paint(&self, page: Page) {
        if self.pages.send(page).is_err() {
            self.last_refresh.set(Some(Err(Fault::new("the painter thread is gone"))));
        }
    }

    pub fn last_refresh(&self) -> LastRefresh {
        self.last_refresh.get()
    }
}
