//! Compatibility boundary between old zksync-os and the new Airbender prover.
//!
//! zksync-os still builds its oracle against the old Airbender revision, while
//! the GPU prover consumes the new `riscv_transpiler` trait. The oracle protocol
//! itself is still a stream of CSR reads/writes plus read-only RAM peeks, so the
//! cheapest bridge is a trait adapter that forwards those calls directly.

use oracle_provider::ZkEENonDeterminismSource;

pub(crate) struct NewAirbenderNonDeterminismSource {
    inner: ZkEENonDeterminismSource,
}

impl From<ZkEENonDeterminismSource> for NewAirbenderNonDeterminismSource {
    fn from(inner: ZkEENonDeterminismSource) -> Self {
        Self { inner }
    }
}

impl airbender_riscv_transpiler::vm::NonDeterminismCSRSource for NewAirbenderNonDeterminismSource {
    fn read(&mut self) -> u32 {
        <ZkEENonDeterminismSource as old_airbender_riscv_transpiler::vm::NonDeterminismCSRSource>::read(
            &mut self.inner,
        )
    }

    fn write_with_memory_access<R: airbender_riscv_transpiler::vm::RamPeek>(
        &mut self,
        ram: &R,
        value: u32,
    ) where
        Self: Sized,
    {
        let ram = NewRamPeekAsOld::new(ram);
        <ZkEENonDeterminismSource as old_airbender_riscv_transpiler::vm::NonDeterminismCSRSource>::write_with_memory_access(
            &mut self.inner,
            &ram,
            value,
        );
    }

    fn write_with_memory_access_dyn(
        &mut self,
        ram: &dyn airbender_riscv_transpiler::vm::RamPeek,
        value: u32,
    ) {
        let ram = NewRamPeekAsOld::new(ram);
        <ZkEENonDeterminismSource as old_airbender_riscv_transpiler::vm::NonDeterminismCSRSource>::write_with_memory_access(
            &mut self.inner,
            &ram,
            value,
        );
    }
}

struct NewRamPeekAsOld<'a, R: airbender_riscv_transpiler::vm::RamPeek + ?Sized> {
    inner: &'a R,
}

impl<'a, R: airbender_riscv_transpiler::vm::RamPeek + ?Sized> NewRamPeekAsOld<'a, R> {
    fn new(inner: &'a R) -> Self {
        Self { inner }
    }
}

impl<R: airbender_riscv_transpiler::vm::RamPeek + ?Sized>
    old_airbender_riscv_transpiler::vm::RamPeek for NewRamPeekAsOld<'_, R>
{
    fn peek_word(&self, address: u32) -> u32 {
        self.inner.peek_word(address)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FixedRam;

    impl airbender_riscv_transpiler::vm::RamPeek for FixedRam {
        fn peek_word(&self, address: u32) -> u32 {
            address ^ 0xa5a5_a5a5
        }
    }

    #[test]
    fn ram_peek_adapter_forwards_words_without_copying() {
        let ram = FixedRam;
        let adapter = NewRamPeekAsOld::new(&ram);

        assert_eq!(
            old_airbender_riscv_transpiler::vm::RamPeek::peek_word(&adapter, 0x1234),
            0xa5a5_b791
        );
    }

    #[test]
    fn disconnected_oracle_reads_zero_through_adapter() {
        let mut source =
            NewAirbenderNonDeterminismSource::from(ZkEENonDeterminismSource::default());

        assert_eq!(
            airbender_riscv_transpiler::vm::NonDeterminismCSRSource::read(&mut source),
            0
        );
    }
}
