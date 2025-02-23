use std::{
    path::PathBuf,
    future::Future,
    time::Duration,
    collections::HashMap,
    sync::{Arc, Mutex},
};

use notify::{Config, Event, RecursiveMode, RecommendedWatcher, Result as NotifyResult, Watcher, EventKind};
use tracing::{info, error, debug, Level};
use crate::app_state::AppState;
use crate::utils::get_relative_path;
use tokio::sync::mpsc::Receiver;
use tokio::task;

// Function to clean the template cache
async fn clean_template_cache(app_state: &AppState) {
    use std::time::Duration;
    let expiration_duration = Duration::from_secs(3600); // 1 hour expiration
                                                           // Clean old templates (e.g., older than 1 hour)
    app_state.clean_old_templates(expiration_duration).await;
}

// Helper function to process events
pub async fn watch_templates(
    template_dir: PathBuf,
    app_state: AppState,
    mut rx: Receiver<Event>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {

    info!("Watching directory in separate task: {:?}", template_dir);

    let mut pending_events: Arc<Mutex<HashMap<PathBuf, EventKind>>> = Arc::new(Mutex::new(HashMap::new()));
    let debounce_duration = Duration::from_millis(200); // Adjust as needed

    while let Some(event) = rx.recv().await {
        debug!("Received event in watch_templates: {:?}", event);
        for path in event.paths {
            if path.extension().map_or(false, |ext| ext == "mjml") {
                let mut events = pending_events.lock().unwrap();
                match event.kind {
                    EventKind::Create(_) | EventKind::Modify(_) => {
                        events.insert(path.clone(), EventKind::Modify(notify::event::ModifyKind::Data(notify::event::DataChange::Any))); // Store modify event
                    }
                    EventKind::Remove(_) => {
                        events.insert(path.clone(), EventKind::Remove(notify::event::RemoveKind::File)); // Store remove event
                    }
                    _ => continue, // Skip Access, Other, Any events
                };
            }
        }

        // Debounce task
        let pending_events_clone = Arc::clone(&pending_events);
        let app_state_clone = app_state.clone();
        let template_dir_clone = template_dir.clone();

        tokio::spawn(async move {
            tokio::time::sleep(debounce_duration).await;

            let mut events = pending_events_clone.lock().unwrap();
            let aggregated_events: Vec<(PathBuf, EventKind)> = events.drain().collect();

            for (path, event_kind) in aggregated_events {
                let app_state_clone2 = app_state_clone.clone();
                let template_dir_clone2 = template_dir_clone.clone();
                let path_clone = path.clone();

                // Calculate the relative path here. Handle errors gracefully.
                let relative_path_result = get_relative_path(&template_dir_clone2, &path_clone);


                match event_kind {
                    EventKind::Create(_) | EventKind::Modify(_) => {
                        info!("Disk updating cache - Reloading template");

                        if let Ok(relative_path) = relative_path_result {
                            let path_str = path_clone.display().to_string();
                            let app_state_clone3 = app_state_clone2.clone();
                            tokio::spawn(async move {
                                if let Err(e) = app_state_clone3.reload_template(&path_str).await {
                                    error!("Failed to reload template {}: {}", path_str, e);
                                }
                            });
                        } else {
                            error!("Failed to get relative path for reload: {:?}", path_clone);
                        }
                    }
                    EventKind::Remove(_) => {
                        info!("Disk updating cache, removing file");

                        if let Ok(relative_path) = relative_path_result {
                            let app_state_clone3 = app_state_clone2.clone();
                            let relative_path_string = relative_path.to_string();

                            tokio::spawn(async move {
                                if let Err(e) = app_state_clone3.remove_template_from_cache(&relative_path_string).await {
                                    error!("Failed to remove template from cache {}: {}", relative_path_string, e);
                                }
                            });
                        } else {
                            error!("Failed to get relative path for removal: {:?}", path_clone);
                        }
                    }
                    _ => continue, // Skip Access, Other, Any events.  This should never happen but kept for safety.
                };

            }
        });
    }
    Ok(())
}
