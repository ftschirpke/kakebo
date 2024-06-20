use crate::{errors::KakeboError, expenses::Expense, KakeboConfig};
use edit::edit;

#[derive(Debug)]
pub struct ExpenseEditor<T> {
    config: KakeboConfig,
    records: Vec<T>,
}

impl<T: Expense> ExpenseEditor<T> {
    pub fn new(config: KakeboConfig) -> Self {
        Self {
            config,
            records: Vec::new(),
        }
    }

    pub fn create_record(&mut self) -> Result<T, KakeboError> {
        let initial_content = T::record_template(&self.records, &self.config);
        let record_string = edit(initial_content)?;
        T::try_create(record_string)
    }
}
