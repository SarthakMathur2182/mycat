use std::io::{self, Cursor, Error, ErrorKind, Read, Write};

struct MyRead<R: Read>(R);

impl<R: Read> Read for MyRead<R> {
    fn read(&mut self, _: &mut [u8]) -> Result<usize, Error> {
        Err(Error::new(ErrorKind::Other, "simulated error"))
    }
}

struct MyWrite(Vec<u8>);
impl Write for MyWrite {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        // Err(Error::new(ErrorKind::Other, "simulated error in write"))
        let mut bytes = 0;
        while bytes < 3 && bytes < buf.len() {
            self.0.push(buf[bytes]);
            bytes += 1;
        }

        println!("write() called: accepted {bytes} bytes");

        Ok(bytes)
    }

    fn flush(&mut self) -> io::Result<()> {
        // Err(Error::new(ErrorKind::Other, "simulated error in flush"))
        // println!("{:?}", self.0);
        // self.0.clear();
        Ok(())
    }
}

fn copy_data<R: Read, W: Write>(reader: &mut R, writer: &mut W) -> io::Result<u64> {
    let mut total_bytes_copied = 0u64;
    let mut buf = [0; 8 << 10];
    loop {
        let bytes_read = reader.read(&mut buf)?;
        if bytes_read == 0 {
            return Ok(total_bytes_copied);
        }

        writer.write_all(&buf[..bytes_read])?;

        // let mut bytes_written = 0;
        // while bytes_written < bytes_read {
        //     let written = writer.write(&buf[bytes_written..bytes_read])?;
        //     println!("Written {} bytes", written);
        //     bytes_written += written;
        // }

        total_bytes_copied += bytes_read as u64;
    }
}

fn main() -> io::Result<()> {
    let mut input = Cursor::new(b"Hello, Rust!".to_vec());
    // let mut input = std::fs::File::open("no_file.txt")?;
    // let mut input = MyRead(input);

    // let mut output = Vec::new();
    let mut output = MyWrite(vec![]);

    // Copy input into output using io::copy.
    // let bytes = io::copy(&mut input, &mut output)?;
    let bytes = copy_data(&mut input, &mut output)?;

    // let mut output2 = vec![];
    // input.set_position(0);
    // // let bytes2 = io::copy(&mut input, &mut output2)?;
    // let bytes2 = copy_data(&mut input, &mut output2)?;

    // Print output as a string.
    println!("Bytes: {}", bytes);
    // println!("Output: {}", String::from_utf8(output).unwrap());
    // println!("Bytes copied again: {}", bytes2);
    // println!("Output again: {}", String::from_utf8(output2).unwrap());

    println!("Output:");
    output.flush()?;

    Ok(())
}
