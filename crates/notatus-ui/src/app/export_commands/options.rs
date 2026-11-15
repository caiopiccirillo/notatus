//! What an export writes: which formats, and whether only finished media.

/// A format the export can write.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::app) enum ExportFormat {
    Yolo,
    Coco,
    Classification,
}

impl ExportFormat {
    pub(in crate::app) const ALL: [Self; 3] = [Self::Yolo, Self::Coco, Self::Classification];

    pub(in crate::app) fn title(self) -> &'static str {
        match self {
            Self::Yolo => "YOLO",
            Self::Coco => "COCO",
            Self::Classification => "Classification",
        }
    }

    /// What the format holds, for the export dialog.
    pub(in crate::app) fn description(self) -> &'static str {
        match self {
            Self::Yolo => "Boxes, in the Ultralytics folder layout",
            Self::Coco => "Boxes and polygons, as one JSON file",
            Self::Classification => "Image labels, one folder per label",
        }
    }
}

/// The export settings, kept for the session. Defaults to every format and
/// finished media only.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::app) struct ExportOptions {
    pub(in crate::app) yolo: bool,
    pub(in crate::app) coco: bool,
    pub(in crate::app) classifications: bool,
    /// Export only media marked done, so unfinished images are not exported
    /// as images with nothing in them.
    pub(in crate::app) finished_only: bool,
}

impl Default for ExportOptions {
    fn default() -> Self {
        Self {
            yolo: true,
            coco: true,
            classifications: true,
            finished_only: true,
        }
    }
}

impl ExportOptions {
    pub(in crate::app) fn includes(self, format: ExportFormat) -> bool {
        match format {
            ExportFormat::Yolo => self.yolo,
            ExportFormat::Coco => self.coco,
            ExportFormat::Classification => self.classifications,
        }
    }

    pub(in crate::app) fn has_format(self) -> bool {
        ExportFormat::ALL
            .into_iter()
            .any(|format| self.includes(format))
    }

    /// Turns `format` on or off, unless it is the last one on. Returns
    /// whether it changed.
    pub(in crate::app) fn toggle(&mut self, format: ExportFormat) -> bool {
        let mut next = *self;
        let slot = match format {
            ExportFormat::Yolo => &mut next.yolo,
            ExportFormat::Coco => &mut next.coco,
            ExportFormat::Classification => &mut next.classifications,
        };
        *slot = !*slot;
        if !next.has_format() {
            return false;
        }
        *self = next;
        true
    }

    /// "YOLO, COCO, and Classification", or the formats that are on.
    pub(in crate::app) fn formats_label(self) -> String {
        let formats: Vec<_> = ExportFormat::ALL
            .into_iter()
            .filter(|format| self.includes(*format))
            .map(ExportFormat::title)
            .collect();
        match formats.as_slice() {
            [] => "None".to_string(),
            [one] => one.to_string(),
            [first, second] => format!("{first} and {second}"),
            [rest @ .., last] => format!("{}, and {last}", rest.join(", ")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toggling_keeps_at_least_one_format() {
        let mut options = ExportOptions::default();
        assert!(options.toggle(ExportFormat::Yolo));
        assert!(options.toggle(ExportFormat::Coco));
        assert!(!options.toggle(ExportFormat::Classification));
        assert!(options.classifications);
        assert_eq!(options.formats_label(), "Classification");
        assert!(options.toggle(ExportFormat::Yolo));
        assert_eq!(options.formats_label(), "YOLO and Classification");
        assert!(options.toggle(ExportFormat::Coco));
        assert_eq!(options.formats_label(), "YOLO, COCO, and Classification");
    }
}
