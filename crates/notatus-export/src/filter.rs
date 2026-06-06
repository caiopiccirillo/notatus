use notatus_core::{AnnotationRecord, ReviewState};
use std::collections::BTreeSet;

#[derive(Clone, Debug)]
pub struct AnnotationFilter {
    review_states: BTreeSet<ReviewState>,
}

impl AnnotationFilter {
    pub fn accepted_and_reviewed() -> Self {
        Self {
            review_states: BTreeSet::from([ReviewState::Accepted, ReviewState::Reviewed]),
        }
    }

    pub fn all_non_rejected() -> Self {
        Self {
            review_states: BTreeSet::from([
                ReviewState::Draft,
                ReviewState::Reviewed,
                ReviewState::Accepted,
            ]),
        }
    }

    pub fn accepts(&self, annotation: &AnnotationRecord) -> bool {
        self.review_states.contains(&annotation.review_state)
    }
}

impl Default for AnnotationFilter {
    fn default() -> Self {
        Self::accepted_and_reviewed()
    }
}
