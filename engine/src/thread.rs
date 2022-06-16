use std::{
    sync::Arc,
    thread::{self, JoinHandle}, time::Instant,
};

use fiddler_base::Game;

use crate::uci::{EngineInfo, UciMessage};

use super::{
    config::SearchConfig, limit::SearchLimit, search::PVSearch, transposition::TTable, SearchError,
    SearchResult,
};

#[derive(Clone, Debug)]
/// The primary search thread for an engine.
pub struct MainSearch {
    pub main_config: SearchConfig,
    configs: Vec<SearchConfig>,
    ttable: Arc<TTable>,
    pub limit: Arc<SearchLimit>,
}

impl MainSearch {
    #[allow(clippy::new_without_default)]
    /// Construct a new main search with only a single search thread.
    pub fn new() -> MainSearch {
        MainSearch {
            main_config: SearchConfig::new(),
            configs: Vec::new(),
            ttable: Arc::new(TTable::default()),
            limit: Arc::new(SearchLimit::new()),
        }
    }

    /// Set the number of helper threads. If `n_helpers` is 0, then this search
    /// will be single-threaded.
    pub fn set_nhelpers(&mut self, n_helpers: usize) {
        if n_helpers >= self.configs.len() {
            for _ in 0..(n_helpers - self.configs.len() + 1) {
                self.configs.push(self.main_config);
            }
        } else {
            self.configs.truncate(n_helpers + 1);
        }
    }

    pub fn evaluate(&self, g: &Game) -> SearchResult {
        let tic = Instant::now();
        let handles: Vec<JoinHandle<SearchResult>> = self
            .configs
            .iter()
            .map(|config| {
                let mut searcher =
                    PVSearch::new(self.ttable.clone(), *config, self.limit.clone(), false);
                let gcopy = g.clone();
                thread::spawn(move || searcher.evaluate(gcopy))
            })
            .collect();

        // now it's our turn to think
        let mut main_searcher = PVSearch::new(
            self.ttable.clone(),
            self.main_config,
            self.limit.clone(),
            true,
        );
        let mut best_result = main_searcher.evaluate(g.clone());

        for handle in handles {
            let eval_result = handle.join().map_err(|_| SearchError::Join)?;

            match (best_result, eval_result) {
                // if this is our first successful thread, use its result
                (Err(_), Ok(_)) => best_result = eval_result,
                // if both were successful, use the deepest result
                (Ok(best_search), Ok(new_search)) => {
                    if new_search.2 > best_search.2 {
                        best_result = eval_result;
                    }
                }
                // error cases cause nothing to happen
                _ => (),
            };
        }
        let toc = Instant::now();
        let elapsed = toc - tic;

        if let Ok((_, _, depth)) = best_result {
            let nodes = self.limit.num_nodes();
            let nps = nodes * 1000 / (elapsed.as_millis() as u64);
            // inform the user
            print!("{}", UciMessage::Info(&[
                EngineInfo::Depth(depth),
                EngineInfo::Nodes(nodes),
                EngineInfo::NodeSpeed(nps),
            ]));
        }

        best_result
    }

    pub fn set_depth(&mut self, depth: u8) {
        self.main_config.depth = depth;
        self.configs
            .iter_mut()
            .for_each(|config| config.depth = depth);
    }
}
