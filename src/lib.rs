pub mod riscv;
pub mod x86;

pub trait Thot {
    #[doc = "Defines the structure of a registry for the target architecture."]
    type Register;
    #[doc = "Defines how a memory address is represented."]
    type MemoryOperand;
    #[doc = "Defines how an immediate value is represented."]
    type Immediate;
    
    /// Correspond au verbe Maât 'HENEK' (L'Offrande / Chargement / Stockage)
    /// Transfère la donnée en garantissant le principe de possession unique.
    fn henek(&mut self, dest: Self::Register, src: Self::MemoryOperand);

    /// Correspond au verbe Maât 'SEMA' (Addition / Transformation Mathématique)
    fn sema(&mut self, dest: Self::Register, src: Self::Register);

    /// Correspond au verbe Maât 'DJA' (Orchestration du flot d'exécution / Transition)
    fn dja(&mut self, target: Self::MemoryOperand);

    /// Correspond au verbe Maât 'WDJ' (Évaluation de seuil / Comparaison)
    fn wdj(&mut self, src1: Self::Register, threshold: Self::Immediate);
}
