#![feature(fmt_helpers_for_derive)]
#![feature(fn_traits)]
#![feature(box_patterns)]

extern crate core;

use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::ops::Deref;
use memmap::Mmap;
use compiler::ast::*;
use compiler::bin_builder::{BinBuilder, JmpType};
use vm::virtual_machine::{Executor, Word};
use crate::compiler::compiler::Compiler;
use crate::variable::Ident;

mod vm;
mod bytecode_examples;
mod compiler;
mod variable;
mod ast_examples;

fn main() {
    let code = ast_examples::while_loop::example();
    //let code = bytecode_examples::for_loop::example();

    {
        let mut file = File::create("test.bin").expect("Could not create file");
        file.write_all(&code).expect("Could not write to file");
        file.flush().expect("Could not flush file");
    }


    let mut vm = Executor {
        stack_frames: vec![],
        stack: vec![],
        program: unsafe { Mmap::map(&File::open("test.bin").expect("Could not open file!")).expect("Could not map file!") },
        externs: vm::bindings::standard_bindings(),
        current_marker: 0
    };

    println!("{:02X?}", vm.program.deref());
    println!("Program length: {} bytes", vm.program.len());
    println!();

    let (ret, time) = vm.run(vec![]);
    println!("execution returned: {:?} ({:?})", ret, time)
}


