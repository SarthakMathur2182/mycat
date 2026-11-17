use std::env::args_os;
use std::fs::File;
use std::io::{Error, ErrorKind, Read, Write, stdin, stdout};
use std::process::ExitCode;

enum MyCatError {
    Read(Error),
    Write(Error),
}

fn copy_reader<R: Read, W: Write>(input: &mut R, output: &mut W) -> Result<(), MyCatError> {
    let mut buf = [0u8; 8 << 10];
    loop {
        let bytes = match input.read(&mut buf) {
            Ok(bytes) => bytes,
            Err(e) if e.kind() == ErrorKind::Interrupted => continue,
            Err(e) => return Err(MyCatError::Read(e)),
        };
        if bytes == 0 {
            break;
        }

        output.write_all(&buf[..bytes]).map_err(MyCatError::Write)?;
    }

    Ok(())
}

fn run() -> Result<bool, MyCatError> {
    let mut args = args_os().peekable();
    args.next()
        .expect("Expecting the target executable (assuming, not verifying).");

    let mut output = stdout().lock();
    let mut success = true;

    if args.peek().is_some() {
        for file in args {
            match File::open(&file) {
                Ok(mut input) => match copy_reader(&mut input, &mut output) {
                    Ok(_) => {}
                    Err(MyCatError::Read(e)) => {
                        eprintln!("File Read error for file {:?}:\n{}", &file, e);
                        success = false;
                    }
                    Err(MyCatError::Write(e)) => {
                        eprintln!("Write error for file {:?}:\n{}", &file, e);
                        return Err(MyCatError::Write(e));
                    }
                },
                Err(e) => {
                    eprintln!("File Open error for file {:?}:\n{}", &file, e);
                    success = false;
                }
            }
        }
    } else {
        let mut input = stdin().lock();
        match copy_reader(&mut input, &mut output) {
            Ok(_) => {}
            Err(MyCatError::Read(e)) => {
                eprintln!("Read error from stdin: {}", e);
                success = false;
            }
            Err(MyCatError::Write(e)) => {
                eprintln!("Write error from stdin: {}", e);
                return Err(MyCatError::Write(e));
            }
        };
    }

    Ok(success)
}

fn main() -> ExitCode {
    match run() {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) | Err(_) => ExitCode::FAILURE,
    }
}

#[cfg(test)]
mod tests {
    use crate::{MyCatError, copy_reader};
    use std::io::{Cursor, Error, ErrorKind, Read, Write};

    #[test]
    fn copies_input_correctly() {
        let mut input = Cursor::new(b"Hello, Rust!");
        let mut output = vec![];
        let result = copy_reader(&mut input, &mut output);
        assert!(result.is_ok());
        assert_eq!(output.as_slice(), b"Hello, Rust!");
    }

    #[test]
    fn handles_empty_input() {
        let mut input = Cursor::new(b"");
        let mut output = vec![];
        let result = copy_reader(&mut input, &mut output);
        assert!(result.is_ok());
        assert!(output.is_empty());
    }

    #[test]
    fn copies_non_utf8_bytes() {
        let mut input = Cursor::new(b"hello \xff\xfe world");
        let mut output = vec![];
        let result = copy_reader(&mut input, &mut output);
        assert!(result.is_ok());
        assert_eq!(output.as_slice(), b"hello \xff\xfe world");
    }

    struct FailingRead;
    impl Read for FailingRead {
        fn read(&mut self, _buf: &mut [u8]) -> std::io::Result<usize> {
            Err(Error::other("mocked read failure"))
        }
    }

    #[test]
    fn returns_read_error_when_reader_fails() {
        let mut input = FailingRead;
        let mut output = vec![];
        let result = copy_reader(&mut input, &mut output);
        assert!(output.is_empty());
        assert!(matches!(result, Err(MyCatError::Read(_))));
    }

    struct FailingWrite;
    impl Write for FailingWrite {
        fn write(&mut self, _buf: &[u8]) -> std::io::Result<usize> {
            Err(Error::other("mocked write failure"))
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn returns_write_error_when_write_fails() {
        let mut input = Cursor::new(b"Hi");
        let mut output = FailingWrite;
        let result = copy_reader(&mut input, &mut output);
        assert!(matches!(result, Err(MyCatError::Write(_))));
    }

    struct PartialWrite(Vec<u8>);
    impl Write for PartialWrite {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            let len = 2.min(buf.len());
            self.0.extend_from_slice(&buf[..len]);
            Ok(len)
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn handles_partial_write() {
        let mut input = Cursor::new(b"hello \xff\xfe world");
        let mut output = PartialWrite(vec![]);
        let result = copy_reader(&mut input, &mut output);
        assert!(result.is_ok());
        assert_eq!(output.0, b"hello \xff\xfe world");
    }

    struct FailAfterPartialWrite {
        buf: Vec<u8>,
        number_of_write_calls: u32,
    }
    impl Write for FailAfterPartialWrite {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            if self.number_of_write_calls >= 2 {
                return Err(Error::other("mocked error after partial write"));
            }
            self.number_of_write_calls += 1;

            let len = 2.min(buf.len());
            self.buf.extend_from_slice(&buf[..len]);
            Ok(len)
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn returns_write_error_on_fail_after_partial_write() {
        let content = b"hello \xff\xfe world";
        let mut input = Cursor::new(content);
        let mut output = FailAfterPartialWrite {
            buf: vec![],
            number_of_write_calls: 0,
        };
        let result = copy_reader(&mut input, &mut output);
        assert!(matches!(result, Err(MyCatError::Write(_))));
        assert_eq!(output.number_of_write_calls, 2);
        assert_eq!(output.buf.as_slice(), &content[..4]);
    }

    struct InterruptedWrite {
        buf: Vec<u8>,
        number_of_write_calls: usize,
    }
    impl Write for InterruptedWrite {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.number_of_write_calls += 1;
            if self.number_of_write_calls == 1 {
                return Err(Error::new(
                    ErrorKind::Interrupted,
                    "mocked interrupted write",
                ));
            }

            let len = 2.min(buf.len());
            self.buf.extend_from_slice(&buf[..len]);
            Ok(len)
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn runs_after_first_write_is_interrupted() {
        let content = b"hello \xff\xfe world";
        let mut input = Cursor::new(content);
        let mut output = InterruptedWrite {
            buf: vec![],
            number_of_write_calls: 0,
        };
        let result = copy_reader(&mut input, &mut output);
        assert!(result.is_ok());
        assert_eq!(output.buf.as_slice(), content);
        assert_eq!(output.number_of_write_calls, content.len().div_ceil(2) + 1); // + 1 for the interrupted call
    }

    struct InterruptedRead {
        input: Cursor<Vec<u8>>,
        number_of_read_calls: usize,
    }

    impl Read for InterruptedRead {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            self.number_of_read_calls += 1;
            if self.number_of_read_calls == 1 {
                return Err(Error::new(
                    ErrorKind::Interrupted,
                    "mocked read interrupt error",
                ));
            }

            self.input.read(buf)
            // let cnt = 3.min(self.input.len());
            // for i in 0..cnt {
            //     buf[i] = self.input.pop().unwrap();
            // }
            // Ok(cnt)
        }
    }

    #[test]
    fn runs_after_first_read_is_interrupted() {
        let mut input = InterruptedRead {
            input: Cursor::new(vec![1, 2, 3, 4, 5, 6, 7]),
            number_of_read_calls: 0,
        };
        let mut output = vec![];
        let result = copy_reader(&mut input, &mut output);
        assert!(result.is_ok());
        assert!(input.number_of_read_calls >= 3);
        assert_eq!(output.as_slice(), [1, 2, 3, 4, 5, 6, 7]);
    }
}
