use crate::Thot;

pub struct R;

impl Thot for R {
    type Register = ();
    type MemoryOperand = ();
    type Immediate = ();

    fn henek(&mut self, dest: Self::Register, src: Self::MemoryOperand) {
        todo!()
    }

    fn sema(&mut self, dest: Self::Register, src: Self::Register) {
        todo!()
    }

    fn dja(&mut self, target: Self::MemoryOperand) {
        todo!()
    }

    fn wdj(&mut self, src1: Self::Register, threshold: Self::Immediate) {
        todo!()
    }
}

