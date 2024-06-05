use std::collections::HashSet;

use anyhow::{bail, Result};
use rand::{rngs::OsRng, Rng};

#[derive(Default)]
pub struct Pins {
    master: u32,
    pins: Vec<Pin>,
    max_id: u8,
}

impl Pins {
    pub fn verify(bytes: &[u8]) -> Result<()> {
        if bytes.is_empty() {
            bail!("Input is empty");
        }
        let len = bytes[0] as usize;
        let bytes = &bytes[1..];
        if bytes.len() < len * 5 {
            bail!("Not enough bytes for given length");
        }
        let mut pins = HashSet::new();
        for i in 0..len {
            let id = bytes[i * 5];
            if id > 99 {
                bail!("Id is too large: {id} > 99");
            }
            if !pins.insert(id) {
                bail!("Duplicate id: {id}");
            }
        }
        Ok(())
    }

    pub fn load(bytes: &[u8], master: u32) -> Self {
        assert!(!bytes.is_empty());
        let len = bytes[0] as usize;
        let bytes = &bytes[1..];
        assert!(bytes.len() >= len * 5);
        let mut pins = Vec::new();
        let mut max_id = 0;
        for i in 0..len {
            let id = bytes[i * 5];
            assert!(id <= 99);
            max_id = max_id.max(id);
            let pin_bytes = &bytes[i * 5 + 1..];
            let pin = (pin_bytes[0] as u32) << 24
                | (pin_bytes[1] as u32) << 16
                | (pin_bytes[2] as u32) << 8
                | pin_bytes[3] as u32;
            let pin = decrypt(master, id, pin);
            pins.push(Pin::new(id, pin));
        }
        pins.sort_by_key(|pin| pin.id);
        Self {
            master,
            pins,
            max_id,
        }
    }

    pub fn save(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.push(self.len() as u8);
        for pin in self.pins.iter().filter(|pin| pin.pin != 0) {
            out.push(pin.id);
            let pin = encrypt(self.master, pin.id, pin.pin);
            out.extend(pin.to_be_bytes());
        }
        out
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn len(&self) -> usize {
        self.pins.len()
    }

    pub fn get(&self, index: usize) -> Pin {
        self.pins[index]
    }

    pub fn set(&mut self, index: usize, value: u32) {
        self.pins[index].pin = value;
    }

    pub fn remove(&mut self, index: usize) {
        self.pins.remove(index);
    }

    pub fn add(&mut self, pin: u32) -> bool {
        if self.max_id >= 99 {
            return false;
        }
        if !self.pins.is_empty() {
            self.max_id += 1;
        }
        self.pins.push(Pin::new(self.max_id, pin));
        true
    }

    pub fn iter(&self) -> std::slice::Iter<Pin> {
        self.pins.iter()
    }
}

#[derive(Clone, Copy)]
pub struct Pin {
    pub id: u8,
    pub pin: u32,
}

impl Pin {
    pub fn new(id: u8, pin: u32) -> Self {
        Self { id, pin }
    }
}

pub fn encrypt(master: u32, id: u8, pin: u32) -> u32 {
    let pin = encapsulate(pin);
    n_shift(master, id + 1) ^ pin
}

pub fn decrypt(master: u32, id: u8, pin: u32) -> u32 {
    let pin = n_shift(master, id + 1) ^ pin;
    decapsulate(pin)
}

pub fn encapsulate(pin: u32) -> u32 {
    let mut x = pin;
    x |= OsRng.gen_range(0b00..=0b11) << 30;
    x
}

pub fn decapsulate(pin: u32) -> u32 {
    let mut x = pin;
    x &= !(0b11 << 30);
    x
}

pub fn n_shift(state: u32, shift: u8) -> u32 {
    let mut x = state;
    for _ in 0..shift {
        x = xorshift32(x);
    }
    x
}

pub fn xorshift32(state: u32) -> u32 {
    let mut x = state;
    x ^= x << 13;
    x ^= x >> 17;
    x ^= x << 5;
    x
}
