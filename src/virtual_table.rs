// src/virtual_table.rs

use crate::data_loader::TableData;

pub struct VirtualTable {
    pub data: TableData,
}

impl VirtualTable {
    pub fn new(data: TableData) -> Self {
        VirtualTable { data }
    }
}