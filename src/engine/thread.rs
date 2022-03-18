use std::{
    sync::{Arc, RwLock},
    thread::{self, JoinHandle},
    time::Instant,
};

use crate::{
    base::Game,
    engine::{branch_factor, search::PVSearch, SearchError, SearchResult},
};

use super::{config::SearchConfig, limit::SearchLimit, transposition::TTable};

#[derive(Clone, Debug)]
/// The primary search thread for an engine.
pub struct MainSearch {
    main_config: SearchConfig,
    configs: Vec<SearchConfig>,
    ttable: Arc<TTable>,
    pub limit: Arc<RwLock<SearchLimit>>,
}

impl MainSearch {
    /// Construct a new main search with only a single search thread.
    pub fn new() -> MainSearch {
        MainSearch {
            main_config: SearchConfig::new(),
            configs: Vec::new(),
            ttable: Arc::new(TTable::default()),
            limit: Arc::new(RwLock::new(SearchLimit::new())),
        }
    }

    /// Set the number of helper threads. If `n_helpers` is 0, then this search
    /// will be single-threaded.
    pub fn set_nhelpers(&mut self, n_helpers: usize) {
        if n_helpers >= self.configs.len() {
            for _ in 0..(n_helpers - self.configs.len() - 1) {
                self.configs.push(self.main_config);
            }
        } else {
            self.configs.truncate(n_helpers - 1);
        }
    }

    pub fn evaluate(&mut self, g: &Game) -> SearchResult {
        let tic = Instant::now(); // start time of the search

        let handles: Vec<JoinHandle<SearchResult>> = self
            .configs
            .iter()
            .map(|config| {
                let mut searcher = PVSearch::new(self.ttable.clone(), *config, self.limit.clone());
                let mut gcopy = g.clone();
                thread::spawn(move || searcher.evaluate(&mut gcopy))
            })
            .collect();

        // now it's our turn to think
        let mut main_searcher =
            PVSearch::new(self.ttable.clone(), self.main_config, self.limit.clone());
        let mut best_result = main_searcher.evaluate(&g);

        for handle in handles {
            let eval_result = handle.join().map_err(|_| SearchError::JoinError)?;

            match (best_result, eval_result) {
                // if this is our first successful thread, use its result
                (Err(_1), Ok(_2)) => best_result = eval_result,
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

        let n_nodes = self
            .limit
            .read()
            .map_err(|_| SearchError::PoisonError)?
            .num_nodes();

        let toc = Instant::now();
        let nsecs = (toc - tic).as_secs_f64();

        if let Ok(search_data) = best_result {
            println!(
                "evaluated {:.0} nodes in {:.0} secs ({:.0} nodes/sec); branch factor {:.2}, hash fill rate {:.2}",
                n_nodes,
                nsecs,
                n_nodes as f64 / nsecs,
                branch_factor(search_data.2, n_nodes),
                self.ttable.fill_rate(),
            );
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
