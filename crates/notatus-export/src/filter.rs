use notatus_core::{AnnotationRecord, AnnotationSource, ClassificationRecord, ReviewState};
use std::collections::BTreeSet;

/// Which annotations and image labels an export includes.
#[derive(Clone, Debug)]
pub struct AnnotationFilter {
    review_states: BTreeSet<ReviewState>,
    /// Whether model predictions nobody has reviewed yet count as drafts.
    model_drafts: bool,
}

impl AnnotationFilter {
    pub fn accepted_and_reviewed() -> Self {
        Self {
            review_states: BTreeSet::from([ReviewState::Accepted, ReviewState::Reviewed]),
            model_drafts: false,
        }
    }

    pub fn all_non_rejected() -> Self {
        Self {
            review_states: BTreeSet::from([
                ReviewState::Draft,
                ReviewState::Reviewed,
                ReviewState::Accepted,
            ]),
            model_drafts: true,
        }
    }

    /// Everything a person made or approved: reviewed and accepted records,
    /// plus drafts drawn by a person or imported. Model predictions still in
    /// draft are left out, because nobody has checked them yet and training
    /// on them would feed the model its own mistakes.
    pub fn ready_for_training() -> Self {
        Self {
            model_drafts: false,
            ..Self::all_non_rejected()
        }
    }

    pub fn accepts(&self, annotation: &AnnotationRecord) -> bool {
        self.accepts_record(&annotation.source, annotation.review_state)
    }

    pub fn accepts_classification(&self, classification: &ClassificationRecord) -> bool {
        self.accepts_record(&classification.source, classification.review_state)
    }

    fn accepts_record(&self, source: &AnnotationSource, review_state: ReviewState) -> bool {
        if review_state == ReviewState::Draft
            && matches!(source, AnnotationSource::Model(_))
            && !self.model_drafts
        {
            return false;
        }
        self.review_states.contains(&review_state)
    }
}

impl Default for AnnotationFilter {
    fn default() -> Self {
        Self::accepted_and_reviewed()
    }
}
