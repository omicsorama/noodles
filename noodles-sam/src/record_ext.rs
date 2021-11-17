use std::io;

use super::{
    header::{ReferenceSequence, ReferenceSequences},
    record::Position,
};

/// SAM(-like) record extensions.
pub trait RecordExt {
    /// Returns the associated reference sequence.
    fn reference_sequence<'rs>(
        &self,
        reference_sequences: &'rs ReferenceSequences,
    ) -> Option<io::Result<&'rs ReferenceSequence>>;

    /// Returns the start position.
    fn alignment_start(&self) -> Option<Position>;

    /// Returns the associated reference sequence of the mate.
    fn mate_reference_sequence<'rs>(
        &self,
        reference_sequences: &'rs ReferenceSequences,
    ) -> Option<io::Result<&'rs ReferenceSequence>>;
}