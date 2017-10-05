#![feature(test)]
#[cfg(test)]
extern crate test;

#[macro_use] extern crate clap;

use std::io::{self, Read, Write, BufRead, BufReader, BufWriter};
use std::fs::{File};
use clap::App;

const BUFSIZE : usize = 65536;

#[derive(Debug, PartialEq, Eq)]
struct DecodeError;

fn main() {
    
    let cli_spec = load_yaml!("cli.yml");
    let cli_args = App::from_yaml(cli_spec).get_matches();

    let mut reader = {
        if let Some(path) = cli_args.value_of("input") {
            Box::new(BufReader::new(match File::open(path) {
                Ok(path) => path,
                _ => {
                    writeln!(io::stderr(), "base💯: no such file: {}", path).expect("base💯: stderr write error");
                    return;
                }
            })) as Box<BufRead>
        } else {
            Box::new(BufReader::new(io::stdin())) as Box<BufRead>
        }
    };

    let mut writer = BufWriter::with_capacity(BUFSIZE, io::stdout());
    if cli_args.is_present("decode") {
        let mut buffer = [0u8; BUFSIZE];
        let mut remain = 0;
        while let Ok(num_read) = reader.read(&mut buffer[remain..remain+1]) {
            if num_read == 0 {
                break;
            }

            let to_process = (remain + num_read) / 4;
            remain = (remain + num_read) % 4;
            for i in 0..to_process {
                if let Err(_) = from_emoticon(&buffer[(i*4)..(i*4+4)])
                    .and_then(|byte|writer.write(&[byte]).map_err(|_|DecodeError)) {
                    writeln!(io::stderr(), "base💯: write error").expect("base💯: stderr write error");
                    std::process::abort();
                }
            }

            for i in 0..remain {
                buffer[i] = buffer[(4*to_process)+i];
            }
        }
    } else {
        let mut write_buf = [0u8; 4*BUFSIZE];
        let mut buffer = [0u8; BUFSIZE];
        while let Ok(num_read) = reader.read(&mut buffer) {
            if num_read == 0 {
                break;
            }

            let buffer = &buffer[0..num_read];
            let write_buf = &mut write_buf[0..(4 * num_read)];
            for i in 0..num_read {
                write_buf[4*i..4*i+4].copy_from_slice(&to_emoticon(buffer[i]));
            }
            match writer.write_all(write_buf) {
                Ok(_) => (),
                _ => {
                    writeln!(io::stderr(), "base💯: write error").expect("base💯: stderr write error");
                    return;
                }
            }
        }
    }
    writer.flush().expect("Write error");
}

#[inline]
fn from_emoticon(mut emo: &[u8]) -> Result<u8, DecodeError> {
    emo = &emo[0..4];
    match (emo[0], emo[1], emo[2], emo[3]) {
        (240, 159, 143, last) if 183 <= last && last < 192 => Ok(last - 183),
        (240, 159, 144, last) if 128 <= last && last < 192 => Ok(last - 128 + 9),
        (240, 159, 145, last) if 128 <= last && last < 192 => Ok(last - 128 + 73),
        (240, 159, 146, last) if 128 <= last && last < 192 => Ok(last - 128 + 137),
        (240, 159, 147, last) if 128 <= last && last < 184 => Ok(last - 128 + 201),
        _ => Err(DecodeError)
    }
}

#[inline]
fn to_emoticon(byte: u8) -> [u8; 4] {
    let mask = if byte < 9 { 255 } else { 0 };
    let byte = byte.wrapping_sub(9);
    let (third_add, fourth_add) = (byte / 64 | mask, byte % 64);
    [240, 159, 144u8.wrapping_add(third_add), 128u8.wrapping_add(fourth_add)]
}

#[cfg(test)]
mod tests {
    const BASE: u32 = 127991;

    fn to_emoticon_original(byte: u8) -> [u8; 4] {
        let mut result = [0; 4];
        ::std::char::from_u32(BASE + (byte as u32)).expect("an emoticon").encode_utf8(&mut result[..]);
        result
    }
    
    fn from_emoticon_original_fast(buf: &[u8]) -> Result<u8, ::DecodeError> {
        unsafe {
            ::std::str::from_utf8_unchecked(&buf)
                .chars()
                .next()
                .map(|c|(c as u32 - BASE) as u8)
                .ok_or(::DecodeError)
        }
    }
    
    fn from_emoticon_original(buf: &[u8]) -> Result<u8, ::DecodeError>  {
        ::std::str::from_utf8(&buf)
            .into_iter()
            .flat_map(str::chars)
            .next()
            .map(|c|(c as u32 - BASE) as u8)
            .ok_or(::DecodeError)
    }

    #[test]
    fn encode_decode_original() {
        for i in 0..256 {
            let expected = Ok(i as u8);
            let buf = to_emoticon_original(i as u8);
            let actual = from_emoticon_original(&buf[..]);
            assert_eq!(expected, actual);
        }
    }
    
    #[test]
    fn encode_decode_new() {
        for i in 0..256 {
            let expected = Ok(i as u8);
            let buf = ::to_emoticon(i as u8);
            let actual = ::from_emoticon(&buf[..]);
            assert_eq!(expected, actual);
        }
    }
    
    #[test]
    fn old_new_equal() {
        for i in 0..256 {
            let expected = to_emoticon_original(i as u8);
            let actual = ::to_emoticon(i as u8);
            assert_eq!(expected, actual);
        }
    }
    
    #[bench]
    fn encode_old(b: &mut ::test::Bencher) {
        let nums = ::test::black_box(0..256).collect::<Vec<_>>();
        b.iter(||{
            let mut result = 0u8;
            for &n in &nums {
                let bytes = to_emoticon_original(n as u8);
                result = result ^ bytes[0] ^ bytes[1] ^ bytes[2] ^ bytes[3];
            }
            
            assert_eq!(28,result);
            result
        });
    }
    
    #[bench]
    fn encode_new(b: &mut ::test::Bencher) {
        let nums = ::test::black_box(0..256).collect::<Vec<_>>();
        b.iter(||{
            let mut result = 0u8;
            for &n in &nums {
                let bytes = ::to_emoticon(n as u8);
                result = result ^ bytes[0] ^ bytes[1] ^ bytes[2] ^ bytes[3];
            }
            
            assert_eq!(28,result);
            result
        });
    }
    
    #[bench]
    fn decode_old(b: &mut ::test::Bencher) {
        let mut input = [0; 256 * 4];
        for n in 0..256 {
            input[4*n..4*n+4].copy_from_slice(&to_emoticon_original(n as u8));
        }
        input = ::test::black_box(input);
        b.iter(||{
            let mut result = 0;
            for n in 0..256 {
                result = result ^ from_emoticon_original(&input[(4*n)..(4*n+4)]).unwrap();
            }
            result
        });
    }
    
    #[bench]
    fn decode_old_fast(b: &mut ::test::Bencher) {
        let mut input = [0; 256 * 4];
        for n in 0..256 {
            input[4*n..4*n+4].copy_from_slice(&to_emoticon_original(n as u8));
        }
        input = ::test::black_box(input);
        b.iter(||{
            let mut result = 0;
            for n in 0..256 {
                result = result ^ from_emoticon_original_fast(&input[(4*n)..(4*n+4)]).unwrap();
            }
            result
        });
    }
    
    #[bench]
    fn decode_new(b: &mut ::test::Bencher) {
        let mut input = [0; 256 * 4];
        for n in 0..256 {
            input[4*n..4*n+4].copy_from_slice(&::to_emoticon(n as u8));
        }
        input = ::test::black_box(input);
        b.iter(||{
            let mut result = 0;
            for n in 0..256 {
                result = result ^ ::from_emoticon(&input[(4*n)..(4*n+4)]).unwrap();
            }
            result
        });
    }
}
