//! Running the project's model runner to suggest annotations.
//!
//! The runner process is kept between runs, so its model loads once. A run
//! sends one request per media item off the UI thread and applies each
//! item's suggestions as they arrive, so they can be reviewed with the
//! review keys while the rest are still coming.

use super::*;
use notatus_infer::{ExternalModelSpec, InferenceRequest, ModelRunner};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

/// Which media a run covers.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum ModelScope {
    CurrentMedia,
    UnfinishedMedia,
}

#[derive(Default)]
pub(super) struct ModelRunState {
    /// The running runner process and the spec it was started from.
    runner: Option<(ExternalModelSpec, Arc<Mutex<ModelRunner>>)>,
    job: Option<ModelJob>,
}

struct ModelJob {
    cancel: Arc<AtomicBool>,
    done: usize,
    total: usize,
    added: usize,
    skipped: usize,
    _task: gpui::Task<()>,
}

impl ModelRunState {
    pub(super) fn is_running(&self) -> bool {
        self.job.is_some()
    }
}

impl NotatusWindow {
    /// Runs the project's model on `scope`, asking for a runner first when
    /// the project has none.
    pub(super) fn run_model(&mut self, scope: ModelScope, cx: &mut Context<Self>) {
        if self.model_run.is_running() {
            self.status_message = Some("A model run is already in progress".to_string());
            return cx.notify();
        }
        let project = self.state.dataset().manifest.project.id;
        let Some(spec) = self.runner_settings.get(project).cloned() else {
            return self.choose_model_runner(Some(scope), cx);
        };
        if self.state.dataset().labels.is_empty() {
            self.status_message =
                Some("Create the labels the model should predict first".to_string());
            return cx.notify();
        }

        let requests = self.model_requests(scope);
        if requests.is_empty() {
            self.status_message = Some(
                match scope {
                    ModelScope::CurrentMedia => "Select an image to run the model on",
                    ModelScope::UnfinishedMedia => "No unfinished images to run the model on",
                }
                .to_string(),
            );
            return cx.notify();
        }

        let runner = match self.model_runner_for(&spec) {
            Ok(runner) => runner,
            Err(error) => {
                self.status_message = Some(format!("Model runner failed to start: {error}"));
                return cx.notify();
            }
        };
        self.start_model_job(runner, requests, cx);
    }

    /// Asks the next run to stop after the media item in progress.
    pub(super) fn stop_model_run(&mut self, cx: &mut Context<Self>) {
        if let Some(job) = &self.model_run.job {
            job.cancel.store(true, Ordering::Relaxed);
            self.status_message = Some("Stopping the model after the current image".to_string());
            cx.notify();
        }
    }

    /// Asks for the program that runs the model, then runs `then` if given.
    pub(super) fn choose_model_runner(&mut self, then: Option<ModelScope>, cx: &mut Context<Self>) {
        self.status_message = Some("Choose the model runner program or script".to_string());
        cx.notify();
        let paths = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some(SharedString::from("Choose model runner")),
        });
        cx.spawn(async move |this, cx| {
            let Ok(Ok(Some(paths))) = paths.await else {
                let _ = this.update(cx, |this, cx| {
                    this.status_message = Some("No model runner chosen".to_string());
                    cx.notify();
                });
                return;
            };
            let Some(path) = paths.into_iter().next() else {
                return;
            };
            let _ = this.update(cx, |this, cx| {
                let spec = runner_spec_for(path);
                let shown = spec
                    .args
                    .first()
                    .cloned()
                    .unwrap_or_else(|| spec.program.to_string_lossy().into_owned());
                this.status_message = Some(format!("Model runner set to {shown}"));
                let project = this.state.dataset().manifest.project.id;
                if let Err(error) = this.runner_settings.set(project, spec) {
                    // Still used for this session.
                    this.status_message =
                        Some(format!("Could not save the model runner choice: {error}"));
                }
                match then {
                    Some(scope) => this.run_model(scope, cx),
                    None => cx.notify(),
                }
            });
        })
        .detach();
    }

    /// One request per media item in `scope` that is stored locally.
    fn model_requests(&self, scope: ModelScope) -> Vec<InferenceRequest> {
        let labels = self.state.dataset().labels.clone();
        self.state
            .dataset()
            .assets
            .iter()
            .filter(|asset| match scope {
                ModelScope::CurrentMedia => self.state.selected_asset() == Some(asset.id),
                ModelScope::UnfinishedMedia => asset.status == notatus_core::AssetStatus::Pending,
            })
            .filter_map(|asset| {
                let AssetLocation::LocalPath { path } = &asset.location else {
                    return None;
                };
                Some(InferenceRequest::object_detection(
                    asset.clone(),
                    labels.clone(),
                    Some(path.clone()),
                ))
            })
            .collect()
    }

    /// The running runner for `spec`, starting it when none runs or the
    /// project's runner changed.
    fn model_runner_for(
        &mut self,
        spec: &ExternalModelSpec,
    ) -> Result<Arc<Mutex<ModelRunner>>, notatus_infer::RunnerError> {
        if let Some((running_spec, runner)) = &self.model_run.runner
            && running_spec == spec
        {
            return Ok(runner.clone());
        }
        let runner = Arc::new(Mutex::new(ModelRunner::spawn(spec)?));
        self.model_run.runner = Some((spec.clone(), runner.clone()));
        Ok(runner)
    }

    fn start_model_job(
        &mut self,
        runner: Arc<Mutex<ModelRunner>>,
        requests: Vec<InferenceRequest>,
        cx: &mut Context<Self>,
    ) {
        let cancel = Arc::new(AtomicBool::new(false));
        let total = requests.len();
        let job_cancel = cancel.clone();
        let task = cx.spawn(async move |this, cx| {
            for request in requests {
                if job_cancel.load(Ordering::Relaxed) {
                    break;
                }
                let runner = runner.clone();
                let result = cx
                    .background_executor()
                    .spawn(async move {
                        let mut runner = runner.lock().unwrap_or_else(|poison| poison.into_inner());
                        let response = runner.infer(&request);
                        (request, response)
                    })
                    .await;
                let failed = this
                    .update(cx, |this, cx| this.apply_model_result(result, cx))
                    .unwrap_or(true);
                if failed {
                    return;
                }
            }
            let _ = this.update(cx, |this, cx| this.finish_model_job(cx));
        });
        self.model_run.job = Some(ModelJob {
            cancel,
            done: 0,
            total,
            added: 0,
            skipped: 0,
            _task: task,
        });
        self.status_message = Some(format!("Model: 0/{total} images"));
        cx.notify();
    }

    /// Applies one media item's response. Returns whether the run failed.
    fn apply_model_result(
        &mut self,
        (request, response): (
            InferenceRequest,
            Result<notatus_infer::InferenceResponse, notatus_infer::RunnerError>,
        ),
        cx: &mut Context<Self>,
    ) -> bool {
        let response = match response {
            Ok(response) => response,
            Err(error) => {
                // A runner that failed may be stuck or gone; start fresh next time.
                self.model_run.runner = None;
                self.model_run.job = None;
                self.status_message = Some(format!("Model run failed: {error}"));
                cx.notify();
                return true;
            }
        };
        let resolved = response.into_annotations(&request);
        let mut skipped = resolved.notes.len();
        let mut added = 0;
        // An error means the media was removed while the model ran.
        if let Ok(summary) = self
            .state
            .apply_model_predictions(request.asset.id, resolved.annotations)
        {
            added = summary.added;
            skipped += summary.skipped;
        }
        if let Some(job) = &mut self.model_run.job {
            job.done += 1;
            job.added += added;
            job.skipped += skipped;
            self.status_message = Some(format!(
                "Model: {}/{} images · {} suggestion{}",
                job.done,
                job.total,
                job.added,
                super::helpers::plural(job.added)
            ));
        }
        cx.notify();
        false
    }

    fn finish_model_job(&mut self, cx: &mut Context<Self>) {
        let Some(job) = self.model_run.job.take() else {
            return;
        };
        let stopped = if job.done < job.total {
            "Stopped: "
        } else {
            ""
        };
        let mut message = format!(
            "{stopped}model suggested {} annotation{} on {} image{}",
            job.added,
            super::helpers::plural(job.added),
            job.done,
            super::helpers::plural(job.done)
        );
        if job.skipped > 0 {
            message.push_str(&format!(
                "; {} prediction{} did not fit the project",
                job.skipped,
                super::helpers::plural(job.skipped)
            ));
        }
        self.status_message = Some(message);
        cx.notify();
    }
}

/// How to run the chosen file: Python scripts through `python3`, so they need
/// not be executable; anything else directly.
fn runner_spec_for(path: PathBuf) -> ExternalModelSpec {
    let is_python = path
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("py"));
    if is_python {
        ExternalModelSpec {
            program: PathBuf::from("python3"),
            args: vec![path.to_string_lossy().into_owned()],
            env: Default::default(),
        }
    } else {
        ExternalModelSpec {
            program: path,
            args: Vec::new(),
            env: Default::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::tests::support::open_test_window;
    use gpui::TestAppContext;

    #[test]
    fn python_scripts_run_through_python() {
        let spec = runner_spec_for(PathBuf::from("/opt/runners/yolo.py"));
        assert_eq!(spec.program, PathBuf::from("python3"));
        assert_eq!(spec.args, ["/opt/runners/yolo.py"]);

        let spec = runner_spec_for(PathBuf::from("/opt/runners/detector"));
        assert_eq!(spec.program, PathBuf::from("/opt/runners/detector"));
        assert!(spec.args.is_empty());
    }

    /// A runner written in sh: it answers every request with one box
    /// labelled with the request's first label.
    #[cfg(unix)]
    fn shell_runner(dir: &Path) -> ExternalModelSpec {
        let script = dir.join("runner.sh");
        std::fs::write(
            &script,
            r#"while IFS= read -r line; do
  id=$(printf '%s' "$line" | sed -n 's/^{"request_id":"\([^"]*\)".*/\1/p')
  name=$(printf '%s' "$line" | sed -n 's/.*"labels":\[{"id":"[^"]*","name":"\([^"]*\)".*/\1/p')
  printf '{"request_id":"%s","model":{"name":"sh"},"predictions":[{"label_name":"%s","geometry":{"kind":"bbox","x":10,"y":10,"width":20,"height":20},"confidence":0.9}]}\n' "$id" "$name"
done
"#,
        )
        .unwrap();
        ExternalModelSpec {
            program: PathBuf::from("sh"),
            args: vec![script.to_string_lossy().into_owned()],
            env: Default::default(),
        }
    }

    #[cfg(unix)]
    #[gpui::test]
    fn running_the_model_adds_suggestions_to_unfinished_media(cx: &mut TestAppContext) {
        let dir = tempfile::tempdir().unwrap();
        let (_, view) = open_test_window(cx);
        let (first, second, done) = view.update(cx, |notatus, _| {
            notatus.state.add_label("Car");
            let first = notatus
                .state
                .add_local_image_asset("/tmp/a.png", 100, 100)
                .unwrap();
            let second = notatus
                .state
                .add_local_image_asset("/tmp/b.png", 100, 100)
                .unwrap();
            let done = notatus
                .state
                .add_local_image_asset("/tmp/c.png", 100, 100)
                .unwrap();
            notatus
                .state
                .set_asset_status(done, notatus_core::AssetStatus::Done)
                .unwrap();
            let project = notatus.state.dataset().manifest.project.id;
            notatus
                .runner_settings
                .set(project, shell_runner(dir.path()))
                .unwrap();
            (first, second, done)
        });

        view.update(cx, |notatus, cx| {
            notatus.run_model(ModelScope::UnfinishedMedia, cx)
        });
        cx.run_until_parked();

        view.update(cx, |notatus, _| {
            let suggested = |asset| {
                notatus
                    .state
                    .dataset()
                    .annotations
                    .iter()
                    .filter(|annotation| annotation.asset_id == asset)
                    .count()
            };
            assert_eq!(
                (suggested(first), suggested(second), suggested(done)),
                (1, 1, 0)
            );
            assert!(!notatus.model_run.is_running());
            assert_eq!(
                notatus.status_message.as_deref(),
                Some("model suggested 2 annotations on 2 images")
            );
        });

        // Running again replaces the untouched suggestions.
        view.update(cx, |notatus, cx| {
            notatus.state.select_asset(first).unwrap();
            notatus.run_model(ModelScope::CurrentMedia, cx)
        });
        cx.run_until_parked();
        view.update(cx, |notatus, _| {
            assert_eq!(notatus.state.dataset().annotations.len(), 2);
        });
    }

    #[gpui::test]
    fn a_runner_that_cannot_start_is_reported(cx: &mut TestAppContext) {
        let (_, view) = open_test_window(cx);
        view.update(cx, |notatus, cx| {
            notatus.state.add_label("Car");
            notatus
                .state
                .add_local_image_asset("/tmp/a.png", 100, 100)
                .unwrap();
            let project = notatus.state.dataset().manifest.project.id;
            let missing = ExternalModelSpec {
                program: PathBuf::from("/nonexistent/runner"),
                args: Vec::new(),
                env: Default::default(),
            };
            notatus.runner_settings.set(project, missing).unwrap();
            notatus.run_model(ModelScope::CurrentMedia, cx);

            let message = notatus.status_message.clone().unwrap();
            assert!(
                message.starts_with("Model runner failed to start"),
                "{message}"
            );
            assert!(!notatus.model_run.is_running());
        });
    }
}
