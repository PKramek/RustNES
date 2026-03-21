use crate::core::bus::Bus;
use crate::core::cartridge::Cartridge;
use crate::core::cpu::{Cpu, CpuError, StepRecord};

#[derive(Debug)]
pub struct Console {
    cpu: Cpu,
    bus: Bus,
}

impl Console {
    pub fn new(cartridge: Cartridge) -> Self {
        Self {
            cpu: Cpu::default(),
            bus: Bus::new(cartridge),
        }
    }

    pub fn cpu(&self) -> &Cpu {
        &self.cpu
    }

    pub fn cpu_mut(&mut self) -> &mut Cpu {
        &mut self.cpu
    }

    pub fn bus(&self) -> &Bus {
        &self.bus
    }

    pub fn bus_mut(&mut self) -> &mut Bus {
        &mut self.bus
    }

    pub fn reset(&mut self) {
        self.cpu.service_reset(&mut self.bus);
    }

    pub fn service_nmi(&mut self) {
        self.cpu.service_nmi(&mut self.bus);
    }

    pub fn service_irq(&mut self) {
        self.cpu.service_irq(&mut self.bus);
    }

    pub fn service_brk(&mut self) {
        self.cpu.service_brk(&mut self.bus);
    }

    pub fn return_from_interrupt(&mut self) {
        self.cpu.return_from_interrupt(&mut self.bus);
    }

    pub fn step_instruction(&mut self) -> Result<StepRecord, CpuError> {
        self.cpu.step_instruction(&mut self.bus)
    }

    pub fn run_with_limit(&mut self, max_instructions: usize) -> Result<Vec<StepRecord>, CpuError> {
        let mut records = Vec::with_capacity(max_instructions);
        for _ in 0..max_instructions {
            records.push(self.step_instruction()?);
        }
        Ok(records)
    }
}