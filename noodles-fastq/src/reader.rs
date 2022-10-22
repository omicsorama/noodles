pub mod records;

pub use self::records::Records;

use std::io::{self, BufRead, Read};

use super::Record;

const LINE_FEED: u8 = b'\n';
const CARRIAGE_RETURN: u8 = b'\r';

/// A FASTQ reader.
pub struct Reader<R> {
    inner: R,
}

impl<R> Reader<R>
where
    R: BufRead,
{
    /// Creates a FASTQ reader.
    ///
    /// # Examples
    ///
    /// ```
    /// use noodles_fastq as fastq;
    /// let data = b"@r0\nATCG\n+\nNDLS\n";
    /// let reader = fastq::Reader::new(&data[..]);
    /// ```
    pub fn new(inner: R) -> Self {
        Self { inner }
    }

    /// Returns a reference to the underlying reader.
    ///
    /// # Examples
    ///
    /// ```
    /// use noodles_fastq as fastq;
    /// let data = [];
    /// let reader = fastq::Reader::new(&data[..]);
    /// assert!(reader.get_ref().is_empty());
    /// ```
    pub fn get_ref(&self) -> &R {
        &self.inner
    }

    /// Returns a mutable reference to the underlying reader.
    ///
    /// # Examples
    ///
    /// ```
    /// use noodles_fastq as fastq;
    /// let data = [];
    /// let mut reader = fastq::Reader::new(&data[..]);
    /// assert!(reader.get_mut().is_empty());
    /// ```
    pub fn get_mut(&mut self) -> &mut R {
        &mut self.inner
    }

    /// Unwraps and returns the underlying reader.
    ///
    /// # Examples
    ///
    /// ```
    /// use noodles_fastq as fastq;
    /// let data = [];
    /// let reader = fastq::Reader::new(&data[..]);
    /// assert!(reader.into_inner().is_empty());
    /// ```
    pub fn into_inner(self) -> R {
        self.inner
    }

    /// Reads a FASTQ record.
    ///
    /// This reads from the underlying stream until four lines are read: the read name, the
    /// sequence, the plus line, and the quality scores. Each line omits the trailing newline.
    ///
    /// The stream is expected to be at the start of a record.
    ///
    /// If successful, the number of bytes read is returned. If the number of bytes read is 0, the
    /// stream reached EOF.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::io;
    /// use noodles_fastq as fastq;
    ///
    /// let data = b"@r0\nATCG\n+\nNDLS\n";
    /// let mut reader = fastq::Reader::new(&data[..]);
    ///
    /// let mut record = fastq::Record::default();
    /// reader.read_record(&mut record)?;
    ///
    /// assert_eq!(record.name(), b"r0");
    /// assert_eq!(record.sequence(), b"ATCG");
    /// assert_eq!(record.quality_scores(), b"NDLS");
    /// Ok::<(), io::Error>(())
    /// ```
    pub fn read_record(&mut self, record: &mut Record) -> io::Result<usize> {
        read_record(&mut self.inner, record)
    }

    /// Returns an iterator over records starting from the current stream position.
    ///
    /// The stream is expected to be at the start of a record.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::io;
    /// use noodles_fastq as fastq;
    ///
    /// let data = b"@r0\nATCG\n+\nNDLS\n";
    /// let mut reader = fastq::Reader::new(&data[..]);
    ///
    /// let mut records = reader.records();
    ///
    /// assert_eq!(
    ///     records.next().transpose()?,
    ///     Some(fastq::Record::new("r0", "ATCG", "NDLS")
    /// ));
    ///
    /// assert!(records.next().is_none());
    /// # Ok::<(), io::Error>(())
    /// ```
    pub fn records(&mut self) -> Records<'_, R> {
        Records::new(self)
    }
}

fn read_record<R>(reader: &mut R, record: &mut Record) -> io::Result<usize>
where
    R: BufRead,
{
    record.clear();

    let mut len = match read_name(reader, record.name_mut()) {
        Ok(0) => return Ok(0),
        Ok(n) => n,
        Err(e) => return Err(e),
    };

    len += read_line(reader, record.sequence_mut())?;
    len += read_description(reader, record.description_mut())?;
    len += read_line(reader, record.quality_scores_mut())?;

    Ok(len)
}

fn read_line<R>(reader: &mut R, buf: &mut Vec<u8>) -> io::Result<usize>
where
    R: BufRead,
{
    match reader.read_until(LINE_FEED, buf) {
        Ok(0) => Ok(0),
        Ok(n) => {
            if buf.ends_with(&[LINE_FEED]) {
                buf.pop();

                if buf.ends_with(&[CARRIAGE_RETURN]) {
                    buf.pop();
                }
            }

            Ok(n)
        }
        Err(e) => Err(e),
    }
}

pub(crate) fn read_name<R>(reader: &mut R, buf: &mut Vec<u8>) -> io::Result<usize>
where
    R: BufRead,
{
    const NAME_PREFIX: u8 = b'@';

    match read_u8(reader) {
        Ok(NAME_PREFIX) => read_line(reader, buf).map(|n| n + 1),
        Ok(_) => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "invalid name prefix",
        )),
        Err(ref e) if e.kind() == io::ErrorKind::UnexpectedEof => Ok(0),
        Err(e) => Err(e),
    }
}

fn read_description<R>(reader: &mut R, buf: &mut Vec<u8>) -> io::Result<usize>
where
    R: BufRead,
{
    const DESCRIPTION_PREFIX: u8 = b'+';

    match read_u8(reader)? {
        DESCRIPTION_PREFIX => read_line(reader, buf).map(|n| n + 1),
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "invalid description prefix",
        )),
    }
}

fn read_u8<R>(reader: &mut R) -> io::Result<u8>
where
    R: Read,
{
    let mut buf = [0; 1];
    reader.read_exact(&mut buf)?;
    Ok(buf[0])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_record() -> io::Result<()> {
        let data = b"\
@noodles:1/1
AGCT
+
abcd
@noodles:2/1
TCGA
+noodles:2/1
dcba
";

        let mut reader = &data[..];
        let mut record = Record::default();

        read_record(&mut reader, &mut record)?;
        let expected = Record::new("noodles:1/1", "AGCT", "abcd");
        assert_eq!(record, expected);

        read_record(&mut reader, &mut record)?;
        let mut expected = Record::new("noodles:2/1", "TCGA", "dcba");
        expected.description_mut().extend_from_slice(b"noodles:2/1");
        assert_eq!(record, expected);

        let n = read_record(&mut reader, &mut record)?;
        assert_eq!(n, 0);

        Ok(())
    }

    #[test]
    fn test_read_line() -> io::Result<()> {
        let mut buf = Vec::new();

        let data = b"noodles\n";
        let mut reader = &data[..];
        buf.clear();
        read_line(&mut reader, &mut buf)?;
        assert_eq!(buf, b"noodles");

        let data = b"noodles\r\n";
        let mut reader = &data[..];
        buf.clear();
        read_line(&mut reader, &mut buf)?;
        assert_eq!(buf, b"noodles");

        let data = b"noodles";
        let mut reader = &data[..];
        buf.clear();
        read_line(&mut reader, &mut buf)?;
        assert_eq!(buf, b"noodles");

        Ok(())
    }

    #[test]
    fn test_read_name() -> io::Result<()> {
        let mut buf = Vec::new();

        let data = b"@r0\n";
        let mut reader = &data[..];
        buf.clear();
        read_name(&mut reader, &mut buf)?;
        assert_eq!(buf, b"r0");

        let data = b"r0\n";
        let mut reader = &data[..];
        buf.clear();
        assert!(matches!(
            read_name(&mut reader, &mut buf),
            Err(ref e) if e.kind() == io::ErrorKind::InvalidData
        ));

        Ok(())
    }

    #[test]
    fn test_read_description() -> io::Result<()> {
        let mut buf = Vec::new();

        let data = b"+r0\n";
        let mut reader = &data[..];
        buf.clear();
        read_description(&mut reader, &mut buf)?;
        assert_eq!(buf, b"r0");

        let data = b"r0\n";
        let mut reader = &data[..];
        buf.clear();
        assert!(matches!(
            read_description(&mut reader, &mut buf),
            Err(ref e) if e.kind() == io::ErrorKind::InvalidData
        ));

        Ok(())
    }
}
