use std::{fmt, thread};

use crate::pins;

pub struct Cracker {
    pins: Box<[RawPin]>,
}

impl Cracker {
    pub fn load(bytes: &[u8]) -> Self {
        assert!(!bytes.is_empty());
        let len = bytes[0] as usize;
        let bytes = &bytes[1..];
        assert!(bytes.len() >= len * 5);
        let mut pins = Vec::new();
        for i in 0..len {
            let id = bytes[i * 5];
            let pin_bytes = &bytes[i * 5 + 1..];
            let raw_pin = (pin_bytes[0] as u32) << 24
                | (pin_bytes[1] as u32) << 16
                | (pin_bytes[2] as u32) << 8
                | pin_bytes[3] as u32;
            pins.push(RawPin { id, pin: raw_pin });
        }
        Self { pins: pins.into() }
    }

    pub fn bruteforce_threaded(&self, thread_count: u32) -> Vec<SusMaster> {
        thread::scope(|scope| {
            let mut handles = Vec::new();
            for i in 0..thread_count {
                handles.push(scope.spawn(move || self.part_bruteforce(i, thread_count, None)));
            }
            handles
                .into_iter()
                .map(thread::ScopedJoinHandle::join)
                .filter_map(|sus| sus.ok())
                .flatten()
                .collect()
        })
    }

    fn part_bruteforce(&self, start: u32, step: u32, max: Option<u32>) -> Vec<SusMaster> {
        let mut sus = Vec::new();
        let mut master = start;
        let max = max.unwrap_or(1_000_000_000);
        while master < max {
            let mut score = 0;
            for raw_pin in self.pins.iter() {
                let pin = pins::decrypt(master, raw_pin.id, raw_pin.pin);
                if pin > 999_999_999 {
                    score = 0;
                    break;
                }
                match pin {
                    0 | 123_456_789 | 987_654_321 => {
                        score += 1;
                    }
                    _ => (),
                }
            }
            if score > 0 {
                sus.push(SusMaster { master, score });
            }
            master += step;
        }
        eprintln!(">> Thread finished.");
        sus
    }

    pub fn find_threaded(&self, thread_count: u32, known_pins: &[u32]) -> Vec<SusMaster> {
        assert!(!known_pins.is_empty());
        thread::scope(|scope| {
            let mut handles = Vec::new();
            for i in 0..thread_count {
                handles
                    .push(scope.spawn(move || self.part_find(i, thread_count, None, known_pins)));
            }
            handles
                .into_iter()
                .map(thread::ScopedJoinHandle::join)
                .filter_map(|sus| sus.ok())
                .flatten()
                .collect()
        })
    }

    fn part_find(
        &self,
        start: u32,
        step: u32,
        max: Option<u32>,
        known_pins: &[u32],
    ) -> Vec<SusMaster> {
        let mut sus = Vec::new();
        let mut master = start;
        let max = max.unwrap_or(1_000_000_000);
        while master < max {
            let mut score = 0;
            for raw_pin in self.pins.iter() {
                let pin = pins::decrypt(master, raw_pin.id, raw_pin.pin);
                if pin > 999_999_999 {
                    score = 0;
                    break;
                }
                if known_pins.contains(&pin) {
                    score += 1;
                }
            }
            if score > 0 {
                sus.push(SusMaster { master, score });
            }
            master += step;
        }
        eprintln!(">> Thread finished.");
        sus
    }
}

/// Encrypted pin
struct RawPin {
    id: u8,
    pin: u32,
}

pub struct SusMaster {
    pub master: u32,
    pub score: u32,
}

impl fmt::Display for SusMaster {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:9} [score={}]", self.master, self.score)
    }
}
