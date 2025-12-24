use memprocfs::{FLAG_NOCACHE, VmmProcess, VmmScatterMemory};
use anyhow::{Ok, Result};
use crate::{constants::unity_offsets, utils::Encoding};

pub struct TarkovVmmProcess<'a> {
    pub scatter: VmmScatterMemory<'a>,
    pub vmm: VmmProcess<'a>,
    pub unity_base: u64
}

impl TarkovVmmProcess<'_> {
    pub fn mem_read_chain(&self, mut ptr: u64, offsets: impl AsRef<[u64]>) -> Result<u64> {
        for offset in offsets.as_ref() {
            ptr = self.vmm.mem_read_as::<u64>(ptr + offset, 0)?;
        }

        return Ok(ptr);
    }

    //Could update this to return byte vec but would need to make my comparision logic handle to handle it. Also would need to handle groupid logic
    pub fn mem_read_string(&self, ptr: u64, length: usize, encoding: Encoding) -> Result<String> {
        let bytes = self.vmm.mem_read_ex(ptr, length, FLAG_NOCACHE)?;
        return Ok(encoding.decode(&bytes)?);
    }

    pub fn get_object_bytes(&self, object_ptr: u64, length: usize) -> Result<Vec<u8>> {
        let name_ptr = self.mem_read_chain(object_ptr, unity_offsets::OBJECT_NAME_CHAIN)?;
        let bytes = self.vmm.mem_read_ex(name_ptr, length, FLAG_NOCACHE)?;
        return Ok(bytes);
    }

    //Make Generic
    pub fn mem_read_array_into_buffer(&self, array_ptr: u64, array_size: usize) -> Result<Vec<u64>> {
        let size_t = size_of::<u64>();
        let total_bytes = array_size * size_t;

        let mut buffer: Vec<u8> = vec![0u8; total_bytes as usize];
        self.vmm.mem_read_into(array_ptr, 0, &mut buffer[..])?;

        let mut ptr_vec = Vec::with_capacity(array_size);
        for i in 0..array_size {
            let start = i * size_t;
            let end = start + size_t;
            let ptr_bytes = &buffer[start..end];
            let ptr = u64::from_ne_bytes(ptr_bytes.try_into()?);
            ptr_vec.push(ptr);
        }

        return Ok(ptr_vec);
    }
}