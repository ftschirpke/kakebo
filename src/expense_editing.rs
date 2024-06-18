use crate::ExpenseType;

#[derive(Debug)]
struct ExpenseEditor {
    expense_type: ExpenseType,
    config: &KakeboConfig,
}

impl ExpenseEditor {
    pub fn new(expense_type: ExpenseType, config: &KakeboConfig) -> Self {
        Self {
            expense_type,
            config,
        }
    }
}
