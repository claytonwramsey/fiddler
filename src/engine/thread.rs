use std::{thread::{self, JoinHandle}, sync::{Arc, RwLock}, time::Instant};

use crate::{engine::{search::PVSearch, SearchResult, SearchError, branch_factor}, base::{MoveGenerator, Game}};

use super::{config::SearchConfig, transposition::TTable, limit::SearchLimit};

#[derive(Clone, Debug)]
/// The primary search thread for an engine.
pub struct MainSearch {
    main_config: SearchConfig,
    configs: Vec<SearchConfig>,
    ttable: Arc<TTable>,
    pub limit: Arc<RwLock<SearchLimit>>,
}

impl MainSearch {

    pub fn new() -> MainSearch {
        MainSearch { 
            main_config: SearchConfig::new(),
            configs: Vec::new(),
            ttable: Arc::new(TTable::default()),
            limit: Arc::new(RwLock::new(SearchLimit::new())),
        }
    }

    /// Set the number of search threads. `nthreads` must be greater than zero.
    /// 
    /// # Panics
    /// 
    /// Panics when `nthreads` is equal to zero.
    pub fn set_nthreads(&mut self, nthreads: usize) {
        assert!(nthreads > 0);
        if nthreads >= self.configs.len() {
            for _ in 0..(nthreads - self.configs.len() - 1) {
                self.configs.push(self.main_config);
            }
        } else {
            self.configs.truncate(nthreads - 1);
        }
    }

    pub fn evaluate(&mut self, g: &Game, mgen: &MoveGenerator) -> SearchResult {
        let tic = Instant::now(); // start time of the search

        let handles: Vec<JoinHandle<SearchResult>> = self.configs.iter().map(|config| {
            let mut searcher = PVSearch::new(
                self.ttable.clone(),
                *config,
                self.limit.clone(),
            );
            let mut gcopy = g.clone();
            let mgencopy = mgen.clone();
            thread::spawn(move || {
                searcher.evaluate(&mut gcopy, &mgencopy)
            })
        }).collect();

        // now it's our turn to think
        let mut main_searcher = PVSearch::new(
            self.ttable.clone(),
            self.main_config,
            self.limit.clone()
        );
        let mut best_result = main_searcher.evaluate(&g, &mgen);

        for handle in handles {
            let eval_result = handle.join().map_err(|_| SearchError::JoinError)?;
            
            match (best_result, eval_result) {
                // if this is our first successful thread, use its result
                (Err(_1), Ok(_2)) => best_result = eval_result,
                // if both were successful, use the deepest result
                (Ok(best_search), Ok(new_search)) => if new_search.2 > best_search.2 {
                    best_result = eval_result;
                }
                // error cases cause nothing to happen
                _ => (),
            };
        }

        let n_nodes = self.limit.read()
            .map_err(|_| SearchError::PoisonError)?.num_nodes();

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
        self.configs.iter_mut().for_each(|config| config.depth = depth);
    }
}