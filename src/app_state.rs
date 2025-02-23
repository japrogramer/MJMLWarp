use std::{
    fs,
    future::Future,
    num::NonZeroUsize,
    path::PathBuf,
    sync::Arc,
    time::{Duration, Instant},
};

use std::env;
use handlebars::Handlebars;
use lru::LruCache;
use notify::{Config, Event, RecursiveMode, RecommendedWatcher, Result as NotifyResult, Watcher, EventKind};

use tokio::sync::RwLock;
use tokio::time::interval;
use tokio::sync::mpsc::channel;
use tokio::fs::read_to_string;

use tracing::{info, debug, error};

use crate::template_watcher::watch_templates;

#[derive(Clone)]
pub struct AppState {
    pub handlebars: Handlebars<'static>,
    template_cache: Arc<RwLock<LruCache<String, CachedTemplate>>>,
    pub template_dir: PathBuf, // Store the template directory
}

struct CachedTemplate {
    pub content: String,
    pub last_accessed: Instant,
}

impl AppState {
    pub fn new(cache_capacity: usize, template_dir: PathBuf) -> Self {
        AppState {
            handlebars: Handlebars::new(),
            template_cache: Arc::new(RwLock::new(LruCache::new(NonZeroUsize::new(cache_capacity).unwrap()))),
            template_dir,
        }
    }

    pub async fn get_template(&self, path: &str) -> Result<String, String> {
        let mut cache = self.template_cache.write().await;
        if let Some(cached) = cache.get_mut(path) {
            cached.last_accessed = Instant::now();
            Ok(cached.content.clone())
        } else {
            // Load the template from disk
            let template_content = read_to_string(path).await
                .map_err(|e| format!("Failed to read template file {}: {}", path, e))?;
            // Store the template in the cache
            cache.put(path.to_string(), CachedTemplate {
                content: template_content.clone(), // Store a clone of the content
                last_accessed: Instant::now(),
            });
            info!("New Template cached.  {} templates cached.", cache.len());
            Ok(template_content) // Return the template content
        }
    }

    pub async fn insert_template(&self, path: String, content: String) {
        let mut cache = self.template_cache.write().await;
        cache.put(path, CachedTemplate {
            content,
            last_accessed: Instant::now(),
        });
    }

    pub async fn clean_old_templates(&self, max_age: Duration) {
        let mut cache = self.template_cache.write().await;
        let now = Instant::now(); // Capture the current time

        let keys_to_remove: Vec<String> = cache
            .iter()
            .filter_map(|(key, template)| {
                if now.duration_since(template.last_accessed) >= max_age {
                    Some(key.clone())
                } else {
                    None
                }
            })
            .collect();

        for key in keys_to_remove {
            cache.pop(&key);
        }
        info!("Template cache cleaned.  {} templates cached.", cache.len());
    }

    pub async fn reload_template(&self, path: &str) -> Result<(), String> {
        if let Ok(template_content) = read_to_string(path).await {
            self.insert_template(path.to_string(), template_content).await;
            info!("Template reloaded: {}", path);
            Ok(())
        } else {
            Err(format!("Failed to reload template: {}", path))
        }
    }


    pub async fn remove_template_from_cache(&self, relative_path: &str) -> Result<(), String> {
        let mut cache = self.template_cache.write().await;
        cache.pop(relative_path);
        info!("Template {} removed from cache.", relative_path);
        Ok(())
    }
}

pub async fn initialize_state(relative_path: &str) -> Result<AppState, Box<dyn std::error::Error + Send + Sync>> {

    // 1. Define templates dir
    // Create a 'templates' directory in your project
    let template_dir = PathBuf::from(format!("{}", relative_path));

    // Create the templates directory if it doesn't exist
    if !template_dir.exists() {
        error!("Directory does not exist: {:?}", template_dir);
        info!("creating Directory: {:?}", template_dir);
        fs::create_dir_all(&template_dir)?; // Handle errors with `?`
    } else {
        info!("Directory: {:?} exists", template_dir);
    }

    // 2. Construct the AppState
    let app_state = AppState::new(100, template_dir.clone());

    // 3. Spawn a background task to clean the cache periodically
    let app_state_clone_0 = app_state.clone(); // Clone for the background task
    tokio::spawn(async move {
        let mut interval = interval(Duration::from_secs(600)); // Every 10 minutes
        loop {
            interval.tick().await; // Wait for the next tick
            clean_template_cache(&app_state_clone_0).await;
        }
    });

    let app_state_clone_1 = app_state.clone();
    let template_dir_clone = template_dir.clone();
    tokio::spawn(async move {
        let (tx, rx) = channel(16); // Use tokio's mpsc channel

        let mut watcher: RecommendedWatcher = match RecommendedWatcher::new(
            move |res: Result<Event, notify::Error>| {
                match res {
                    Ok(event) => {
                        if let Err(e) = tx.blocking_send(event) {
                            error!("Error sending event: {}", e);
                        }
                    }
                    Err(e) => error!("watch error: {:?}", e),
                }
            },
            Config::default().with_poll_interval(Duration::from_millis(100)).with_compare_contents(true),
        ) {
            Ok(watcher) => watcher,
            Err(e) => {
                error!("Failed to create watcher: {}", e);
                 // It's critical to log the error and potentially handle it more gracefully.
                return; // Important: Exit the spawned task on error
            }
        };


        info!("watch directory: {}", template_dir_clone.display()); // Use the cloned path
        if let Err(e) = watcher.watch(&template_dir_clone, RecursiveMode::Recursive) {
            error!("Failed to watch directory: {}", e);
            return; // Exit the spawned task if watching fails
        }

        if let Err(e) = watch_templates(template_dir_clone.clone(), app_state_clone_1, rx).await {
            error!("Error in watch_templates task: {}", e);
        }
    });

    Ok(app_state) // Return the `AppState` wrapped in `Result`
}

// Function to clean the template cache
async fn clean_template_cache(app_state: &AppState) {
    let expiration_duration = Duration::from_secs(3600); // 1 hour expiration
                                                           // Clean old templates (e.g., older than 1 hour)
    app_state.clean_old_templates(expiration_duration).await;
}
