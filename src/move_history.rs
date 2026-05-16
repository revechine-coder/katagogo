use crate::game_state::MoveRecord;

#[derive(Debug, Clone)]
pub struct MoveHistory {
    records: Vec<MoveRecord>,
    current_index: usize,
}

impl MoveHistory {
    pub fn new() -> Self {
        MoveHistory {
            records: Vec::new(),
            current_index: 0,
        }
    }

    pub fn push(&mut self, record: MoveRecord) {
        if self.current_index < self.records.len() {
            self.records.truncate(self.current_index);
        }
        self.records.push(record);
        self.current_index = self.records.len();
    }

    pub fn undo(&mut self) -> Option<MoveRecord> {
        if self.current_index == 0 {
            return None;
        }
        self.current_index -= 1;
        Some(self.records[self.current_index].clone())
    }

    pub fn redo(&mut self) -> Option<MoveRecord> {
        if self.current_index >= self.records.len() {
            return None;
        }
        let record = self.records[self.current_index].clone();
        self.current_index += 1;
        Some(record)
    }

    pub fn jump_to(&mut self, move_number: u32) -> Vec<MoveRecord> {
        if move_number == 0 {
            let undone: Vec<MoveRecord> = self.records[..self.current_index].to_vec();
            self.current_index = 0;
            return undone;
        }
        if (move_number as usize) > self.records.len() {
            self.current_index = self.records.len();
            return vec![];
        }
        if (move_number as usize) < self.current_index {
            let undone: Vec<MoveRecord> =
                self.records[move_number as usize..self.current_index].to_vec();
            self.current_index = move_number as usize;
            undone
        } else {
            self.current_index = move_number as usize;
            vec![]
        }
    }

    pub fn len(&self) -> usize {
        self.records.len()
    }

    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    pub fn current_index(&self) -> usize {
        self.current_index
    }

    pub fn last(&self) -> Option<&MoveRecord> {
        if self.current_index == 0 {
            return None;
        }
        self.records.get(self.current_index - 1)
    }

    pub fn all_records(&self) -> &[MoveRecord] {
        &self.records
    }

    pub fn records_up_to_current(&self) -> &[MoveRecord] {
        &self.records[..self.current_index]
    }

    pub fn reset(&mut self) {
        self.records.clear();
        self.current_index = 0;
    }
}
