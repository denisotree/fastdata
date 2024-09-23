// src/data_loader.rs

use std::error::Error;


#[derive(Debug)]
pub struct TableData {
    pub headers: Vec<String>,
    pub columns: Vec<Vec<String>>,
}

impl TableData {
    pub fn new(headers: Vec<String>, columns: Vec<Vec<String>>) -> Self {
        TableData { headers, columns }
    }
}


pub trait DataLoader {
    fn load(&self, path: &str) -> Result<TableData, Box<dyn Error>>;
}


pub struct CsvLoader;

impl DataLoader for CsvLoader {
    fn load(&self, path: &str) -> Result<TableData, Box<dyn Error>> {
        let mut reader = csv::Reader::from_path(path)?;
        let headers = reader
            .headers()?
            .iter()
            .map(String::from)
            .collect::<Vec<String>>();

        let mut columns: Vec<Vec<String>> = headers.iter().map(|_| Vec::new()).collect();

        for result in reader.records() {
            let record = result?;
            for (i, field) in record.iter().enumerate() {
                columns[i].push(field.to_string());
            }
        }

        Ok(TableData::new(headers, columns))
    }
}


pub fn get_loader(extension: &str) -> Result<Box<dyn DataLoader>, Box<dyn Error>> {
    match extension.to_lowercase().as_str() {
        "csv" => Ok(Box::new(CsvLoader)),

        _ => Err(format!("File format '{}' is not supported", extension).into()),
    }
}