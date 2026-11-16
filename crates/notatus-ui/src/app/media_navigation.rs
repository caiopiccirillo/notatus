//! Moving between media and marking them done.

use crate::Direction;
use crate::app::helpers::plural;
use crate::app::window::NotatusWindow;
use gpui::Context;
use notatus_core::{AssetId, AssetStatus};

impl NotatusWindow {
    /// Selects a media item and resets the canvas for it.
    pub(super) fn go_to_media(&mut self, asset_id: AssetId, cx: &mut Context<Self>) {
        if self.state.selected_asset() == Some(asset_id) {
            return;
        }
        match self.state.select_asset(asset_id) {
            Ok(()) => {
                self.tools.fit_canvas_to_view();
                self.canvas_image_layout.borrow_mut().take();
                self.hovered_annotation = None;
                self.status_message = None;
            }
            Err(error) => self.status_message = Some(error.to_string()),
        }
        cx.notify();
    }

    pub(super) fn step_media(
        &mut self,
        direction: Direction,
        pending_only: bool,
        cx: &mut Context<Self>,
    ) {
        match self
            .state
            .adjacent_asset(self.state.selected_asset(), direction, pending_only)
        {
            Some(asset_id) => self.go_to_media(asset_id, cx),
            None => {
                self.status_message = Some(
                    match (direction, pending_only) {
                        (_, true) => "No more pending media",
                        (Direction::Next, false) => "Already at the last media",
                        (Direction::Previous, false) => "Already at the first media",
                    }
                    .to_string(),
                );
                cx.notify();
            }
        }
    }

    pub(super) fn toggle_media_done(&mut self, cx: &mut Context<Self>) {
        let Some(asset) = self.selected_asset() else {
            return;
        };
        let (asset_id, status) = match asset.status {
            AssetStatus::Done => (asset.id, AssetStatus::Pending),
            AssetStatus::Pending => (asset.id, AssetStatus::Done),
        };
        match self.state.set_asset_status(asset_id, status) {
            Ok(()) => {
                self.status_message = Some(match status {
                    AssetStatus::Done => "Marked media done".to_string(),
                    AssetStatus::Pending => "Marked media pending".to_string(),
                });
            }
            Err(error) => self.status_message = Some(error.to_string()),
        }
        cx.notify();
    }

    /// Marks the current media done and moves on to the next pending one.
    pub(super) fn done_and_next(&mut self, cx: &mut Context<Self>) {
        let Some(asset_id) = self.state.selected_asset() else {
            return;
        };
        if let Err(error) = self.state.set_asset_status(asset_id, AssetStatus::Done) {
            self.status_message = Some(error.to_string());
            cx.notify();
            return;
        }
        let next = self
            .state
            .adjacent_asset(Some(asset_id), Direction::Next, true)
            .or_else(|| self.state.adjacent_asset(None, Direction::Next, true));
        match next {
            Some(next) => self.go_to_media(next, cx),
            None => {
                self.status_message = Some("All media done".to_string());
                cx.notify();
            }
        }
    }

    pub(super) fn remove_asset(&mut self, asset_id: AssetId, cx: &mut Context<Self>) {
        match self.state.remove_asset(asset_id) {
            Ok(removed) => {
                self.hovered_annotation = None;
                self.tools.fit_canvas_to_view();
                self.status_message = Some(format!(
                    "Removed media, {} annotation{}, and {} classification{}",
                    removed.annotations,
                    plural(removed.annotations),
                    removed.classifications,
                    plural(removed.classifications),
                ));
            }
            Err(error) => self.status_message = Some(error.to_string()),
        }
        cx.notify();
    }
}
