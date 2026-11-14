use std::fs::File;
use std::io::{Error, ErrorKind, Read, Write, stdout};
use std::process::ExitCode;

struct MyRead;
impl Read for MyRead {
    fn read(&mut self, _buf: &mut [u8]) -> std::io::Result<usize> {
        Err(Error::new(ErrorKind::Other, "mocked failure"))
    }
}

enum MyCatError {
    Read(Error),
    Write(Error),
}

fn copy_reader<R: Read, W: Write>(input: &mut R, output: &mut W) -> Result<(), MyCatError> {
    let mut buf = [0u8; 8 << 10];
    loop {
        match input.read(&mut buf) {
            Ok(bytes) => {
                if bytes == 0 {
                    break;
                }

                output.write_all(&buf[..bytes]).map_err(MyCatError::Write)?;
            }
            Err(e) => {
                return Err(MyCatError::Read(e));
            }
        }
    }

    Ok(())
}

fn run() -> Result<bool, MyCatError> {
    let mut output = stdout().lock();
    let mut success = true;

    for file in ["text_file.txt", "missing.txt", "text_file2.txt"] {
        match File::open(file) {
            Ok(mut input) => match copy_reader(&mut input, &mut output) {
                Ok(_) => {}
                Err(MyCatError::Read(e)) => {
                    eprintln!("File Read error for file {}:\n{}", file, e);
                    success = false;
                }
                Err(MyCatError::Write(e)) => {
                    eprintln!("Write error for file {}:\n{}", file, e);
                    return Err(MyCatError::Write(e));
                }
            },
            Err(e) => {
                eprintln!("File Open error for file {}:\n{}", file, e);
                success = false;
            }
        }
    }

    let mut input = MyRead;
    match copy_reader(&mut input, &mut output) {
        Ok(_) => {}
        Err(MyCatError::Read(e)) => {
            eprintln!("Read error {}", e);
            success = false;
        }
        Err(MyCatError::Write(e)) => {
            eprintln!("Write error {}", e);
            return Err(MyCatError::Write(e));
        }
    };

    Ok(success)
}

fn main() -> ExitCode {
    match run() {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) | Err(_) => ExitCode::FAILURE,
    }
}
