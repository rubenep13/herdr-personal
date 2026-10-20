//! The colour strips the theme pickers draw beside each theme's name: the
//! first eight colours of its palette, loaded off the UI thread a batch at a
//! time for the rows on screen, and kept in a small cache.
use crate::HerdrWindow;
use gpui::Context;
use std::collections::VecDeque;

/// How many strips the cache keeps, and how many one load reads.
const CACHE_LIMIT: usize = 64;
const BATCH: usize = 12;

/// The cache, one load at a time. `None` in an entry means the theme has no
/// palette to show (it failed to load).
#[derive(Debug, Default)]
pub(crate) struct ThemeSwatches {
    epoch: u64,
    running: bool,
    queued: bool,
    strips: VecDeque<(String, Option<[u32; 8]>)>,
}

impl ThemeSwatches {
    /// `None` while `name` has not been loaded; `Some(None)` when it has no
    /// palette.
    pub(crate) fn get(&self, name: &str) -> Option<Option<[u32; 8]>> {
        self.strips
            .iter()
            .find(|(cached, _)| cached == name)
            .map(|(_, strip)| *strip)
    }

    /// The first of `names` not in the cache, as many as one load reads.
    pub(crate) fn wanted(&self, names: &[String]) -> Vec<String> {
        names
            .iter()
            .filter(|name| self.get(name).is_none())
            .take(BATCH)
            .cloned()
            .collect()
    }

    /// Claims the one load slot: `None` while a load is running.
    pub(crate) fn start(&mut self) -> Option<u64> {
        if self.running {
            return None;
        }
        self.running = true;
        Some(self.epoch)
    }

    /// Keeps a load's strips unless the cache was emptied since it started.
    pub(crate) fn finish(&mut self, epoch: u64, strips: Vec<(String, Option<[u32; 8]>)>) {
        self.running = false;
        if self.epoch != epoch {
            return;
        }
        for strip in strips {
            if self.strips.len() == CACHE_LIMIT {
                self.strips.pop_front();
            }
            self.strips.push_back(strip);
        }
    }

    /// Forgets every strip; a load still running is dropped when it ends.
    pub(crate) fn invalidate(&mut self) {
        self.epoch = self.epoch.wrapping_add(1);
        self.strips.clear();
    }
}

impl HerdrWindow {
    /// Loads the strips of the `names` a picker shows, if any are missing.
    /// Called while rendering, so the load starts after this frame.
    pub(crate) fn schedule_theme_swatches(&mut self, names: Vec<String>, cx: &mut Context<Self>) {
        let swatches = &mut self.menu.theme_swatches;
        if swatches.queued || swatches.running || swatches.wanted(&names).is_empty() {
            return;
        }
        swatches.queued = true;
        let weak = cx.entity().downgrade();
        cx.defer(move |cx| {
            let _ = weak.update(cx, |this, cx| {
                this.menu.theme_swatches.queued = false;
                this.load_theme_swatches(names, cx);
            });
        });
    }

    fn load_theme_swatches(&mut self, names: Vec<String>, cx: &mut Context<Self>) {
        let wanted = self.menu.theme_swatches.wanted(&names);
        if wanted.is_empty() {
            return;
        }
        let Some(epoch) = self.menu.theme_swatches.start() else {
            return;
        };
        let mut config = self.config.clone();
        let light = crate::app::light_appearance(cx);
        let work = cx.background_executor().spawn(async move {
            wanted
                .into_iter()
                .map(|name| {
                    config.theme = name.clone();
                    let strip = config
                        .theme(light)
                        .ok()
                        .and_then(|theme| <[u32; 8]>::try_from(&theme.palette[..8]).ok());
                    (name, strip)
                })
                .collect::<Vec<_>>()
        });
        cx.spawn(async move |this, cx| {
            let strips = work.await;
            let _ = this.update(cx, |this, cx| {
                this.menu.theme_swatches.finish(epoch, strips);
                // The next batch, while the same rows are on screen.
                this.load_theme_swatches(names, cx);
                cx.notify();
            });
        })
        .detach();
    }
}

#[cfg(test)]
mod tests;
