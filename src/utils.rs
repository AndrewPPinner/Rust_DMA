use anyhow::{Ok, Result};
use serde::Serialize;

pub enum Encoding {
    UFT8,
    UNICODE
}

impl Encoding {
    pub fn decode(&self, bytes: &[u8]) -> Result<String> {
        match self {
            Encoding::UFT8 => {
                let len = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
                let slice = &bytes[..len];

                Ok(std::str::from_utf8(slice)?.to_owned())
            },
            Encoding::UNICODE => {
                let mut len = bytes.len();
                for i in (0..bytes.len()).step_by(2) {
                    if bytes[i] == 0 && bytes[i + 1] == 0 {
                        len = i;
                        break;
                    }
                }

                let trimmed = &bytes[..len];

                let utf16: Vec<u16> = trimmed
                    .chunks_exact(2)
                    .map(|c| u16::from_le_bytes([c[0], c[1]]))
                    .collect();

                Ok(String::from_utf16(&utf16)?)
            },
        }
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Serialize)]
pub struct Vector2 {
    pub x: f32,
    pub y: f32,
}