use globset::{Glob, GlobSet, GlobSetBuilder};
use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver};
use std::time::{Duration, Instant};

use crate::config::WatchConfig;

struct ServiceWatchRules {
    root_dir: PathBuf,
    include_globs: Option<GlobSet>,
    exclude_globs: Option<GlobSet>,
    ignore_paths: Vec<PathBuf>,
    debounce_duration: Duration,
    last_trigger: Option<Instant>,
    pending_event: bool,
    pending_since: Option<Instant>,
}

pub struct ServiceWatcher {
    _watcher: RecommendedWatcher,
    event_rx: Receiver<notify::Result<Event>>,
    services: HashMap<String, ServiceWatchRules>,
    watched_paths: HashMap<PathBuf, usize>, // Path -> ref count
}

impl ServiceWatcher {
    pub fn new() -> Result<Self, notify::Error> {
        let (tx, rx) = channel();

        let watcher = RecommendedWatcher::new(
            move |res| {
                let _ = tx.send(res);
            },
            Config::default(),
        )?;

        Ok(Self {
            _watcher: watcher,
            event_rx: rx,
            services: HashMap::new(),
            watched_paths: HashMap::new(),
        })
    }

    pub fn watch_service(
        &mut self,
        service_name: &str,
        cwd: &Path,
        config: &WatchConfig,
    ) -> Result<(), String> {
        let root = if cwd.is_absolute() {
            cwd.to_path_buf()
        } else {
            std::env::current_dir()
                .unwrap_or_else(|_| PathBuf::from("."))
                .join(cwd)
        };
        let canonical_root = root.canonicalize().unwrap_or(root);

        // Build include globs
        let include_globs = if config.include.is_empty() {
            None
        } else {
            let mut builder = GlobSetBuilder::new();
            for pattern in &config.include {
                let glob = Glob::new(pattern)
                    .map_err(|e| format!("Invalid include glob '{pattern}': {e}"))?;
                builder.add(glob);
            }
            Some(
                builder
                    .build()
                    .map_err(|e| format!("Failed to compile include globs: {e}"))?,
            )
        };

        // Build exclude globs
        let exclude_globs = if config.exclude.is_empty() {
            None
        } else {
            let mut builder = GlobSetBuilder::new();
            for pattern in &config.exclude {
                let glob = Glob::new(pattern)
                    .map_err(|e| format!("Invalid exclude glob '{pattern}': {e}"))?;
                builder.add(glob);
            }
            Some(
                builder
                    .build()
                    .map_err(|e| format!("Failed to compile exclude globs: {e}"))?,
            )
        };

        // Parse ignore paths
        let mut ignore_paths: Vec<PathBuf> = config
            .ignore_paths
            .iter()
            .map(|p| canonical_root.join(p))
            .collect();
        // Common default ignore paths
        ignore_paths.push(canonical_root.join(".git"));
        ignore_paths.push(canonical_root.join("target"));
        ignore_paths.push(canonical_root.join("node_modules"));

        // Register watch on directory if not already watched
        if !self.watched_paths.contains_key(&canonical_root) {
            if let Err(e) = self
                ._watcher
                .watch(&canonical_root, RecursiveMode::Recursive)
            {
                eprintln!(
                    "Warning: failed to watch directory {}: {e}",
                    canonical_root.display()
                );
            }
            self.watched_paths.insert(canonical_root.clone(), 1);
        } else {
            *self.watched_paths.get_mut(&canonical_root).unwrap() += 1;
        }

        self.services.insert(
            service_name.to_string(),
            ServiceWatchRules {
                root_dir: canonical_root,
                include_globs,
                exclude_globs,
                ignore_paths,
                debounce_duration: Duration::from_millis(config.debounce_ms.max(50)),
                last_trigger: None,
                pending_event: false,
                pending_since: None,
            },
        );

        Ok(())
    }

    pub fn unwatch_service(&mut self, service_name: &str) {
        if let Some(rules) = self.services.remove(service_name) {
            if let Some(count) = self.watched_paths.get_mut(&rules.root_dir) {
                *count = count.saturating_sub(1);
                if *count == 0 {
                    let _ = self._watcher.unwatch(&rules.root_dir);
                    self.watched_paths.remove(&rules.root_dir);
                }
            }
        }
    }

    /// Drain incoming filesystem events and return a list of service names that should restart.
    pub fn poll_changed_services(&mut self) -> Vec<String> {
        let now = Instant::now();

        // Drain channel events
        while let Ok(res) = self.event_rx.try_recv() {
            if let Ok(event) = res {
                // Filter out access or metadata-only reads
                if matches!(
                    event.kind,
                    EventKind::Access(_) | EventKind::Other
                ) {
                    continue;
                }

                for path in &event.paths {
                    self.match_and_flag_path(path, now);
                }
            }
        }

        // Check debouncing for flagged services
        let mut ready_services = Vec::new();
        for (name, rules) in &mut self.services {
            if rules.pending_event {
                if let Some(since) = rules.pending_since {
                    if now.duration_since(since) >= rules.debounce_duration {
                        rules.pending_event = false;
                        rules.pending_since = None;
                        rules.last_trigger = Some(now);
                        ready_services.push(name.clone());
                    }
                }
            }
        }

        ready_services
    }

    fn match_and_flag_path(&mut self, path: &Path, now: Instant) {
        for rules in self.services.values_mut() {
            // Check if path is within this service's root
            if !path.starts_with(&rules.root_dir) {
                continue;
            }

            // Check ignore paths
            if rules.ignore_paths.iter().any(|ignored| path.starts_with(ignored)) {
                continue;
            }

            let rel_path = match path.strip_prefix(&rules.root_dir) {
                Ok(p) => p,
                Err(_) => continue,
            };

            // Check exclude globs
            if let Some(ref exclude) = rules.exclude_globs {
                if exclude.is_match(rel_path) {
                    continue;
                }
            }

            // Check include globs
            if let Some(ref include) = rules.include_globs {
                if !include.is_match(rel_path) {
                    continue;
                }
            }

            // Path matched this service! Flag for debouncing
            rules.pending_event = true;
            if rules.pending_since.is_none() {
                rules.pending_since = Some(now);
            }
        }
    }
}
