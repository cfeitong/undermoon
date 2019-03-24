use std::io;
use std::sync;
use std::iter;
use std::cmp;
use futures::{future, Future, Async, Poll};
use tokio::io::AsyncWrite;


struct CircularBuffer {
    buf: Vec<u8>,
    start: usize,
    end: usize,
}

impl CircularBuffer {
    fn new(capacity: usize) -> Self {
        assert_gt!(capacity, 0);
        Self {
            buf: vec![0; capacity],  // TODO: might use Vec::set_len
            start: 0,
            end: 0,
        }
    }

    fn empty(&self) -> bool { self.start == self.end }

    fn write_data(&mut self, data: &[u8]) -> usize {
        let buf_size = self.buf.len();
        if self.start <= self.end {
            let queue_size = if self.start == 0 { buf_size - 1 } else { buf_size };
            debug_assert_ge!(queue_size, self.end);
            let first_len = cmp::min(queue_size - self.end, data.len());

            debug_assert_ge!(self.end, 0);
            debug_assert_le!(self.end + first_len, buf_size);
            debug_assert_le!(first_len, data.len());
            self.buf[self.end..self.end+first_len].copy_from_slice(&data[0..first_len]);
            self.end += first_len;
            debug_assert_le!(self.end, buf_size);
            if self.end == buf_size {
                assert_gt!(self.start, 0);
                self.end = 0;
            }

            // full or done
            if (self.end + 1) % buf_size == self.start || first_len == data.len() {
                return first_len;
            }

            debug_assert_gt!(self.start, 0);
            debug_assert_ge!(data.len(), first_len);
            let second_len = cmp::min(self.start - 1, data.len() - first_len);

            debug_assert_lt!(second_len, self.start);
            debug_assert_le!(self.start, buf_size);
            debug_assert_ge!(first_len, 0);
            debug_assert_le!(first_len+second_len, data.len());
            self.buf[0..second_len].copy_from_slice(&data[first_len..first_len+second_len]);
            assert_eq!(self.end, 0);
            self.end = second_len;
            assert_lt!(self.end, self.start);
            assert_lt!(self.end, buf_size);
            first_len + second_len
        } else {
            debug_assert_ge!(self.start - self.end, 1);
            let len = cmp::min(self.start - 1 - self.end, data.len());

            debug_assert_ge!(self.end, 0);
            debug_assert_lt!(self.end + len, self.start);
            debug_assert_le!(self.end + len, buf_size);
            debug_assert_le!(len, data.len());
            self.buf[self.end..self.end+len].copy_from_slice(&data[0..len]);

            self.end += len;
            // todo: wrap end
            debug_assert_le!(self.end, buf_size);
            assert_lt!(self.start, buf_size);
            assert_lt!(self.end, buf_size);
            len
        }
    }

    fn writable_len(&self) -> usize {
        if self.start <= self.end {
            let first_len = self.buf.len() - self.end;
            if self.start > 1 {
                first_len + self.start - 1
            } else {
                first_len
            }
        } else {
            self.start - self.end - 1
        }
    }

    fn write_to<W: std::io::Write>(&mut self, writer: &mut W) -> std::io::Result<usize> {
        let start = self.start;
        let end = self.end;
        let buf_size = self.buf.len();
        if self.start <= self.end {
            self.write_range_to(writer, start, end)
        } else {
            let first_len = match self.write_range_to(writer, start, buf_size) {
                Ok(written) => written,
                err => return err,
            };
            match self.write_range_to(writer, 0, end) {
                Ok(written) => Ok(first_len + written),
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    Ok(first_len)
                }
                Err(e) => Err(e),
            }
        }
    }

    fn write_range_to<W: std::io::Write>(&mut self, writer: &mut W, start: usize, end: usize) -> std::io::Result<usize> {
        if self.empty() {
            return Ok(0);
        }
        match writer.write(&self.buf[start..end]) {
            Ok(written) => {
                self.start += written;
                if self.start == self.buf.len() {
                    self.start = 0;
                }
                debug_assert_lt!(self.end, self.buf.len());
                debug_assert_lt!(self.start, self.buf.len());
                Ok(written)
            },
            err => err,
        }
    }
}


pub struct CircularBufWriter<W: AsyncWrite> {
    inner_writer: W,
    cir_buf: CircularBuffer,
}

impl<W: AsyncWrite> CircularBufWriter<W> {
    pub fn new(writer: W, capacity: usize) -> Self {
        assert_gt!(capacity, 0);
        Self {
            inner_writer: writer,
            cir_buf: CircularBuffer::new(capacity),
        }
    }
}

impl<W: AsyncWrite> std::io::Write for CircularBufWriter<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let cir_buf = &mut self.cir_buf;
        let inner_writer = &mut self.inner_writer;
        if cir_buf.writable_len() < buf.len() {
            if let Err(e) = cir_buf.write_to(inner_writer) {
                return Err(e)
            }
        }
        Ok(cir_buf.write_data(buf))
    }

    fn flush(&mut self) -> io::Result<()> {
        let cir_buf = &mut self.cir_buf;
        let inner_writer = &mut self.inner_writer;
        match cir_buf.write_to(inner_writer) {
            Ok(_) => {
                if !cir_buf.empty() {
                    return Err(std::io::Error::new(std::io::ErrorKind::WouldBlock, "CircularBufWriter flush not done"));
                }
            }
            Err(e) => return Err(e),
        }
        inner_writer.flush()
    }
}

impl<W: AsyncWrite> AsyncWrite for CircularBufWriter<W> {
    fn shutdown(&mut self) -> Poll<(), std::io::Error> {
        if !self.cir_buf.empty() {
            match self.poll_flush() {
                Ok(Async::Ready(())) => (),
                poll_result => return poll_result,
            }
        }
        self.inner_writer.shutdown()
    }
}
