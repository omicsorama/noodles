//! BAM CIGAR and operations.

pub mod op;
mod ops;

pub use self::{op::Op, ops::Ops};

use std::{
    fmt, io,
    ops::{Deref, DerefMut},
};

use noodles_sam::{self as sam, record::cigar::op::Kind};

/// BAM record CIGAR.
#[derive(Clone, Default, Eq, PartialEq)]
pub struct Cigar(Vec<u32>);

impl Cigar {
    /// Creates a CIGAR by wrapping raw CIGAR data.
    ///
    /// # Examples
    ///
    /// ```
    /// use noodles_bam::record::Cigar;
    /// let data = vec![0x00000240, 0x00000084]; // 36M8S
    /// let cigar = Cigar::from(data);
    /// assert_eq!(**cigar, [0x00000240, 0x00000084]);
    /// ```
    #[deprecated(since = "0.8.0", note = "Use `Cigar::from::<Vec<u32>>` instead.")]
    pub fn new(cigar: Vec<u32>) -> Cigar {
        Self::from(cigar)
    }

    /// Returns an iterator over the operations in the CIGAR.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::io;
    /// use noodles_bam::record::{cigar::Op, Cigar};
    /// use noodles_sam::record::cigar::op::Kind;
    ///
    /// let data = vec![0x00000240, 0x00000084]; // 36M8S
    /// let cigar = Cigar::from(data);
    ///
    /// let mut ops = cigar.ops();
    ///
    /// assert_eq!(ops.next().transpose()?, Some(Op::new(Kind::Match, 36)?));
    /// assert_eq!(ops.next().transpose()?, Some(Op::new(Kind::SoftClip, 8)?));
    /// assert_eq!(ops.next().transpose()?, None);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn ops(&self) -> Ops<'_> {
        Ops::new(&self.0)
    }

    /// Calculates the alignment span over the reference sequence.
    ///
    /// This sums the lengths of the CIGAR operations that consume the reference sequence, i.e.,
    /// alignment matches (`M`), deletions from the reference (`D`), skipped reference regions
    /// (`S`), sequence matches (`=`), and sequence mismatches (`X`).
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::io;
    /// use noodles_bam::record::{cigar::Op, Cigar};
    /// use noodles_sam::record::cigar::op::Kind;
    ///
    /// let data = vec![0x00000240, 0x00000043, 0x00000084]; // 36M4D8S
    /// let cigar = Cigar::from(data);
    ///
    /// assert_eq!(cigar.reference_len()?, 40);
    /// # Ok::<(), io::Error>(())
    /// ```
    pub fn reference_len(&self) -> io::Result<u32> {
        let mut len = 0;

        for result in self.ops() {
            let op = result?;

            match op.kind() {
                Kind::Match | Kind::Deletion | Kind::Skip | Kind::SeqMatch | Kind::SeqMismatch => {
                    len += op.len();
                }
                _ => {}
            }
        }

        Ok(len)
    }

    /// Calculates the read length.
    ///
    /// This sums the lengths of the CIGAR operations that consume the read, i.e., alignment
    /// matches (`M`), insertions to the reference (`I`), soft clips (`S`), sequence matches (`=`),
    /// and sequence mismatches (`X`).
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::io;
    /// use noodles_bam::record::{cigar::Op, Cigar};
    /// use noodles_sam::record::cigar::op::Kind;
    ///
    /// let data = vec![0x00000240, 0x00000043, 0x00000084]; // 36M4D8S
    /// let cigar = Cigar::from(data);
    ///
    /// assert_eq!(cigar.read_len()?, 44);
    /// # Ok::<(), io::Error>(())
    /// ```
    pub fn read_len(&self) -> io::Result<u32> {
        let mut len = 0;

        for result in self.ops() {
            let op = result?;

            match op.kind() {
                Kind::Match
                | Kind::Insertion
                | Kind::SoftClip
                | Kind::SeqMatch
                | Kind::SeqMismatch => {
                    len += op.len();
                }
                _ => {}
            }
        }

        Ok(len)
    }
}

impl<'a> fmt::Debug for Cigar {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.debug_list().entries(self.ops()).finish()
    }
}

impl<'a> fmt::Display for Cigar {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for result in self.ops() {
            let op = result.map_err(|_| fmt::Error)?;
            write!(f, "{}", op)?;
        }

        Ok(())
    }
}

impl Deref for Cigar {
    type Target = Vec<u32>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Cigar {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl From<Vec<u32>> for Cigar {
    fn from(cigar: Vec<u32>) -> Self {
        Self(cigar)
    }
}

impl TryFrom<&Cigar> for sam::record::Cigar {
    type Error = io::Error;

    fn try_from(cigar: &Cigar) -> Result<Self, Self::Error> {
        let mut ops = Vec::new();

        for result in cigar.ops() {
            let op = result?;
            ops.push(sam::record::cigar::Op::new(op.kind(), op.len()));
        }

        Ok(Self::from(ops))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_bytes() -> Result<(), Box<dyn std::error::Error>> {
        let data = vec![0x00000240, 0x00000362]; // 36MD54
        let cigar = Cigar::from(data);

        let mut ops = cigar.ops();

        assert_eq!(ops.next().transpose()?, Some(Op::try_from(0x240)?));
        assert_eq!(ops.next().transpose()?, Some(Op::try_from(0x362)?));
        assert_eq!(ops.next().transpose()?, None);

        Ok(())
    }

    #[test]
    fn test_try_from_cigar_for_sam_record_cigar() -> io::Result<()> {
        use sam::record::cigar::{op, Op};

        let data = vec![0x00000240, 0x00000362]; // 36MD54
        let cigar = Cigar::from(data);

        let actual = sam::record::Cigar::try_from(&cigar)?;
        let expected = sam::record::Cigar::from(vec![
            Op::new(op::Kind::Match, 36),
            Op::new(op::Kind::Deletion, 54),
        ]);

        assert_eq!(actual, expected);

        Ok(())
    }
}
