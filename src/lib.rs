#![allow(dead_code)]

#[macro_use]
extern crate bitflags;

pub mod cpu;
pub mod cart;

pub trait MMU {
    fn read(&self, a: u16) -> u8;
    fn write(&mut self, a: u16, v: u8);
    fn cycle(&mut self);
}