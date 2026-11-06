use crate::model::AgentGraph;
use crate::source::{
    CommandExecutor, DEFAULT_MAX_LINE_BYTES, HcomSyncWorker, StreamItem, StreamKillHandle,
    StreamReader, build_messages_follow_cmd, parse_stream_line,
};
use anyhow::Result;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

pub struct BackgroundSync {
    shared_graph: Arc<Mutex<Option<AgentGraph>>>,
    dirty: Arc<AtomicBool>,
    shutdown: Arc<AtomicBool>,
    kill_handle: Arc<Mutex<Option<Arc<dyn StreamKillHandle>>>>,
    worker_thread: Option<JoinHandle<()>>,
    pub published_count: Arc<AtomicUsize>,
}

impl BackgroundSync {
    pub fn new<E: CommandExecutor + 'static>(
        worker: HcomSyncWorker<E>,
        batch_limit: usize,
        cadence: Duration,
    ) -> Result<Self> {
        let shared_graph = Arc::new(Mutex::new(None));
        let dirty = Arc::new(AtomicBool::new(false));
        let shutdown = Arc::new(AtomicBool::new(false));
        let kill_handle: Arc<Mutex<Option<Arc<dyn StreamKillHandle>>>> = Arc::new(Mutex::new(None));
        let published_count = Arc::new(AtomicUsize::new(0));

        let shared_graph_clone = Arc::clone(&shared_graph);
        let dirty_clone = Arc::clone(&dirty);
        let shutdown_clone = Arc::clone(&shutdown);
        let kill_handle_clone = Arc::clone(&kill_handle);
        let published_count_clone = Arc::clone(&published_count);

        let worker_thread = std::thread::spawn(move || {
            let mut local_graph = AgentGraph::new();
            if worker.initial_sync(&mut local_graph).is_ok() {
                let mut guard = shared_graph_clone.lock().unwrap();
                *guard = Some(local_graph.clone());
                dirty_clone.store(true, Ordering::Release);
                published_count_clone.fetch_add(1, Ordering::Relaxed);
            }

            let mut reader: Option<Box<dyn StreamReader>> = None;
            let mut pending_events = 0usize;
            let mut last_publish = Instant::now();

            let publish_state = |graph: &AgentGraph,
                                 shared: &Arc<Mutex<Option<AgentGraph>>>,
                                 dirty: &Arc<AtomicBool>,
                                 counter: &Arc<AtomicUsize>| {
                let mut guard = shared.lock().unwrap();
                *guard = Some(graph.clone());
                dirty.store(true, Ordering::Release);
                counter.fetch_add(1, Ordering::Relaxed);
            };

            while !shutdown_clone.load(Ordering::SeqCst) {
                // If no active reader, spawn follower split
                if reader.is_none() {
                    let cmd = build_messages_follow_cmd(local_graph.last_event_id);
                    match worker.executor.spawn_stream_split(&cmd) {
                        Ok((r, k)) => {
                            *kill_handle_clone.lock().unwrap() = Some(k);
                            reader = Some(r);
                        }
                        Err(_) => {
                            // Backoff delay with shutdown checks
                            for _ in 0..10 {
                                if shutdown_clone.load(Ordering::SeqCst) {
                                    break;
                                }
                                std::thread::sleep(Duration::from_millis(50));
                            }
                            continue;
                        }
                    }
                }

                if let Some(ref mut r) = reader {
                    match r.next_line(DEFAULT_MAX_LINE_BYTES) {
                        Ok(Some(line)) => {
                            match parse_stream_line(&line) {
                                StreamItem::Message(msg) => {
                                    if local_graph.apply_message_event(&msg) {
                                        pending_events += 1;
                                    }
                                }
                                StreamItem::DbReset => {
                                    // Kill existing follower before reset
                                    if let Some(k) = kill_handle_clone.lock().unwrap().take() {
                                        let _ = k.kill();
                                    }
                                    reader = None;
                                    local_graph.reset_events();
                                    let _ = worker.initial_sync(&mut local_graph);
                                    publish_state(
                                        &local_graph,
                                        &shared_graph_clone,
                                        &dirty_clone,
                                        &published_count_clone,
                                    );
                                    pending_events = 0;
                                    last_publish = Instant::now();
                                    continue;
                                }
                                StreamItem::Ignored => {}
                            }

                            // Throttled publish check
                            if pending_events > 0
                                && (pending_events >= batch_limit
                                    || last_publish.elapsed() >= cadence)
                            {
                                publish_state(
                                    &local_graph,
                                    &shared_graph_clone,
                                    &dirty_clone,
                                    &published_count_clone,
                                );
                                pending_events = 0;
                                last_publish = Instant::now();
                            }
                        }
                        Ok(None) | Err(_) => {
                            // Stream closed or error
                            if pending_events > 0 {
                                publish_state(
                                    &local_graph,
                                    &shared_graph_clone,
                                    &dirty_clone,
                                    &published_count_clone,
                                );
                                pending_events = 0;
                                last_publish = Instant::now();
                            }

                            if let Some(k) = kill_handle_clone.lock().unwrap().take() {
                                let _ = k.kill();
                            }
                            reader = None;

                            // Small backoff before reconnection
                            for _ in 0..5 {
                                if shutdown_clone.load(Ordering::SeqCst) {
                                    break;
                                }
                                std::thread::sleep(Duration::from_millis(50));
                            }
                        }
                    }
                }
            }

            // Flush any remaining updates on exit
            if pending_events > 0 {
                publish_state(
                    &local_graph,
                    &shared_graph_clone,
                    &dirty_clone,
                    &published_count_clone,
                );
            }
        });

        Ok(Self {
            shared_graph,
            dirty,
            shutdown,
            kill_handle,
            worker_thread: Some(worker_thread),
            published_count,
        })
    }

    /// Checks if a new graph state is available without blocking.
    /// If dirty, moves the latest graph out instead of cloning it: the graph
    /// holds the whole message history, so a clone here would stall the UI
    /// thread (~12 ms per publish at ~14k messages).
    pub fn try_get_latest(&self) -> Option<AgentGraph> {
        if self.dirty.swap(false, Ordering::AcqRel) {
            let mut guard = self.shared_graph.lock().unwrap();
            guard.take()
        } else {
            None
        }
    }

    /// Returns the current graph unconditionally (if any snapshot has been
    /// produced and not yet taken by `try_get_latest`).
    pub fn current_graph(&self) -> Option<AgentGraph> {
        let guard = self.shared_graph.lock().unwrap();
        guard.clone()
    }
}

impl Drop for BackgroundSync {
    fn drop(&mut self) {
        self.shutdown.store(true, Ordering::SeqCst);
        if let Some(k) = self.kill_handle.lock().unwrap().take() {
            let _ = k.kill();
        }
        if let Some(handle) = self.worker_thread.take() {
            let _ = handle.join();
        }
    }
}
