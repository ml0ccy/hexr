use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum EditOperation {
    /// Вставка байта: позиция, старое значение, новое значение
    InsertByte { position: usize, old_value: Option<u8>, new_value: u8 },
    /// Удаление байта: позиция, старое значение
    DeleteByte { position: usize, old_value: u8 },
    /// Замена байта: позиция, старое значение, новое значение
    ReplaceByte { position: usize, old_value: u8, new_value: u8 },
    /// Вставка блока байтов: позиция, старые значения, новые значения
    InsertBytes { position: usize, old_values: Vec<u8>, new_values: Vec<u8> },
    /// Удаление блока байтов: позиция, старые значения
    DeleteBytes { position: usize, old_values: Vec<u8> },
    /// Замена блока байтов: позиция, старые значения, новые значения
    ReplaceBytes { position: usize, old_values: Vec<u8>, new_values: Vec<u8> },
}

impl EditOperation {
    pub fn new_replace_byte(position: usize, old_value: u8, new_value: u8) -> Self {
        Self::ReplaceByte { position, old_value, new_value }
    }

    pub fn undo(&self, data: &mut Vec<u8>) {
        match self {
            EditOperation::InsertByte { position, old_value, .. } => {
                if let Some(old_val) = old_value {
                    data[*position] = *old_val;
                } else {
                    data.remove(*position);
                }
            }
            EditOperation::DeleteByte { position, old_value } => {
                data.insert(*position, *old_value);
            }
            EditOperation::ReplaceByte { position, old_value, .. } => {
                data[*position] = *old_value;
            }
            EditOperation::InsertBytes { position, old_values, .. } => {
                if old_values.is_empty() {
                    data.truncate(*position);
                } else {
                    let current_len = data.len();
                    let end_pos = (*position + old_values.len()).min(current_len);
                    data.splice(*position..end_pos, old_values.iter().cloned());
                    // Если вставили больше, чем было, усекаем
                    if end_pos < current_len {
                        data.truncate(current_len - (current_len - end_pos) + old_values.len());
                    }
                }
            }
            EditOperation::DeleteBytes { position, old_values } => {
                data.splice(*position..*position, old_values.iter().cloned());
            }
            EditOperation::ReplaceBytes { position, old_values, .. } => {
                data.splice(*position..*position + old_values.len(), old_values.iter().cloned());
            }
        }
    }

    pub fn redo(&self, data: &mut Vec<u8>) {
        match self {
            EditOperation::InsertByte { position, new_value, .. } => {
                if *position < data.len() {
                    data[*position] = *new_value;
                } else {
                    data.push(*new_value);
                }
            }
            EditOperation::DeleteByte { position, .. } => {
                if *position < data.len() {
                    data.remove(*position);
                }
            }
            EditOperation::ReplaceByte { position, new_value, .. } => {
                if *position < data.len() {
                    data[*position] = *new_value;
                }
            }
            EditOperation::InsertBytes { position, new_values, .. } => {
                let current_len = data.len();
                if *position <= current_len {
                    data.splice(*position..*position, new_values.iter().cloned());
                } else {
                    // Если позиция за пределами массива, добавляем в конец
                    data.extend_from_slice(new_values);
                }
            }
            EditOperation::DeleteBytes { position, old_values } => {
                let current_len = data.len();
                if *position < current_len {
                    let len = old_values.len();
                    data.drain(*position..(*position + len).min(current_len));
                }
            }
            EditOperation::ReplaceBytes { position, new_values, .. } => {
                data.splice(*position..*position + new_values.len(), new_values.iter().cloned());
            }
        }
    }
}

pub struct UndoRedoStack {
    undo_stack: Vec<EditOperation>,
    redo_stack: Vec<EditOperation>,
    max_operations: usize,
}

impl UndoRedoStack {
    pub fn new(max_operations: usize) -> Self {
        Self {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            max_operations,
        }
    }

    pub fn push(&mut self, operation: EditOperation) {
        self.undo_stack.push(operation);
        self.redo_stack.clear(); // Очищаем redo стек при новой операции

        // Ограничиваем размер стека
        if self.undo_stack.len() > self.max_operations {
            self.undo_stack.remove(0);
        }
    }

    pub fn undo(&mut self) -> Option<EditOperation> {
        self.undo_stack.pop().inspect(|op| {
            self.redo_stack.push(op.clone());
        })
    }

    pub fn redo(&mut self) -> Option<EditOperation> {
        self.redo_stack.pop().inspect(|op| {
            self.undo_stack.push(op.clone());
        })
    }

    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    pub fn clear(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
    }

}

impl Default for UndoRedoStack {
    fn default() -> Self {
        Self::new(1000) // Максимум 1000 операций
    }
}
